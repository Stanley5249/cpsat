//! Acquires OR-Tools, compiles the C ABI bridge, and emits link directives.
//!
//! `CPSAT_ORTOOLS` selects where OR-Tools comes from: `prebuilt` (default),
//! `source`, or `dir:<path>`. An environment variable replaces Cargo features
//! because features are additive and two modes cannot both apply.

#[path = "build/artifacts.rs"]
mod artifacts;
#[path = "build/fetch.rs"]
mod fetch;
#[path = "build/source.rs"]
mod source;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

enum Mode {
    Prebuilt,
    Source,
    Dir(PathBuf),
}

enum Toolchain {
    Msvc,
    Gnu,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    for path in ["build.rs", "build", "native"] {
        println!("cargo:rerun-if-changed={path}");
    }
    for var in ["CPSAT_ORTOOLS", "CPSAT_CACHE_DIR", "DOCS_RS"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    // docs.rs builds without network, and rustdoc never links.
    if env::var_os("DOCS_RS").is_some() {
        return Ok(());
    }

    let target = env::var("TARGET")?;
    let out = PathBuf::from(env::var("OUT_DIR")?);
    let toolchain = toolchain(&target)?;
    let root = match mode()? {
        Mode::Prebuilt => {
            let archive = artifacts::prebuilt(&target).with_context(|| {
                format!(
                    "no prebuilt OR-Tools {} for {target}; set CPSAT_ORTOOLS=source or \
                     CPSAT_ORTOOLS=dir:<path>",
                    artifacts::VERSION
                )
            })?;
            fetch::extracted(archive, &fetch::cache_dir()?)?
        }
        Mode::Source => {
            let cache = fetch::cache_dir()?;
            let source = fetch::extracted(&artifacts::SOURCE, &cache)?;
            source::build(&source, &cache.join("source").join(&target))?
        }
        Mode::Dir(path) => {
            println!("cargo:rerun-if-changed={}", path.display());
            path
        }
    };
    if !root.join("include/ortools/sat/swig_helper.h").exists() {
        bail!("{} is not an OR-Tools C++ install", root.display());
    }

    let runtime = out.join("runtime");
    copy_runtime(&root, &runtime, &toolchain)?;
    compile_bridge(&root, &toolchain)?;
    link(&root, &runtime, &toolchain)?;

    // Dependents read these as DEP_ORTOOLS_ROOT and DEP_ORTOOLS_RUNTIME, for
    // example to copy the runtime libraries next to a shipped executable.
    println!("cargo:root={}", root.display());
    println!("cargo:runtime={}", runtime.display());
    Ok(())
}

fn mode() -> Result<Mode> {
    let value = env::var("CPSAT_ORTOOLS").unwrap_or_default();
    match value.as_str() {
        "" | "prebuilt" => Ok(Mode::Prebuilt),
        "source" => Ok(Mode::Source),
        _ => value
            .strip_prefix("dir:")
            .map(|path| Mode::Dir(PathBuf::from(path)))
            .with_context(|| {
                format!("unknown CPSAT_ORTOOLS={value:?}; expected prebuilt, source, or dir:<path>")
            }),
    }
}

fn toolchain(target: &str) -> Result<Toolchain> {
    if target.ends_with("-windows-msvc") {
        Ok(Toolchain::Msvc)
    } else if target.contains("-linux-gnu") {
        Ok(Toolchain::Gnu)
    } else {
        bail!("unsupported target {target}; expected *-windows-msvc or *-linux-gnu")
    }
}

/// Copies the shared libraries under `OUT_DIR`, because Cargo puts native
/// search paths inside the target directory on `PATH` or `LD_LIBRARY_PATH` for
/// `cargo run` and `cargo test`.
fn copy_runtime(root: &Path, runtime: &Path, toolchain: &Toolchain) -> Result<()> {
    if runtime.exists() {
        fs::remove_dir_all(runtime)?;
    }
    fs::create_dir_all(runtime)?;
    let (dir, wanted): (_, fn(&str) -> bool) = match toolchain {
        Toolchain::Msvc => ("bin", |name| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
        }),
        Toolchain::Gnu => ("lib", |name| name.contains(".so")),
    };
    for entry in fs::read_dir(root.join(dir))? {
        let entry = entry?;
        let name = entry.file_name();
        if !name.to_str().is_some_and(wanted) {
            continue;
        }
        let to = runtime.join(&name);
        if entry.file_type()?.is_symlink() {
            copy_symlink(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn copy_symlink(from: &Path, to: &Path) -> Result<()> {
    std::os::unix::fs::symlink(fs::read_link(from)?, to)?;
    Ok(())
}

// Windows archives contain no symbolic links, so a copy of the target is
// equivalent.
#[cfg(not(unix))]
fn copy_symlink(from: &Path, to: &Path) -> Result<()> {
    fs::copy(from, to)?;
    Ok(())
}

fn compile_bridge(root: &Path, toolchain: &Toolchain) -> Result<()> {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("native/bridge.cc")
        .include("native")
        .include(root.join("include"));
    // Match the definitions OR-Tools exports to its CMake consumers, because
    // some of them change declarations in the installed headers.
    for define in exported_definitions(root)? {
        match define.split_once('=') {
            Some((name, value)) => build.define(name, value),
            None => build.define(&define, None),
        };
    }
    match toolchain {
        Toolchain::Msvc => {
            build
                .std("c++20")
                .flags(["/Zc:preprocessor", "/Zc:__cplusplus", "/bigobj", "/utf-8"])
                .define("NOMINMAX", None)
                .define("WIN32_LEAN_AND_MEAN", "1")
                .define("PROTOBUF_USE_DLLS", None)
                .define("ABSL_CONSUME_DLL", None);
        }
        Toolchain::Gnu => {
            build.std("c++17").flag("-fwrapv");
        }
    }
    build.compile("cpsat_bridge");
    Ok(())
}

/// Reads `INTERFACE_COMPILE_DEFINITIONS` of `ortools::ortools` from the
/// installed CMake package.
fn exported_definitions(root: &Path) -> Result<Vec<String>> {
    let targets = root.join("lib/cmake/ortools/ortoolsTargets.cmake");
    let text = fs::read_to_string(&targets)
        .with_context(|| format!("failed to read {}", targets.display()))?;
    let block = text
        .split("set_target_properties(ortools::ortools PROPERTIES")
        .nth(1)
        .and_then(|rest| rest.split("\n)").next())
        .with_context(|| format!("no ortools::ortools target in {}", targets.display()))?;
    let definitions = block
        .lines()
        .find_map(|line| line.trim().strip_prefix("INTERFACE_COMPILE_DEFINITIONS "))
        .map(|value| value.trim_matches('"'))
        .unwrap_or_default();
    Ok(definitions
        .split(';')
        .filter(|define| !define.is_empty() && !define.contains("$<"))
        .map(str::to_owned)
        .collect())
}

fn link(root: &Path, runtime: &Path, toolchain: &Toolchain) -> Result<()> {
    println!("cargo:rustc-link-search=native={}", runtime.display());
    match toolchain {
        Toolchain::Msvc => {
            println!(
                "cargo:rustc-link-search=native={}",
                root.join("lib").display()
            );
            for lib in ["ortools", "libprotobuf", "abseil_dll"] {
                println!("cargo:rustc-link-lib=dylib={lib}");
            }
        }
        Toolchain::Gnu => {
            // The bridge calls Protobuf and Abseil directly, and GNU ld rejects
            // symbols that resolve only through another shared library. rustc
            // links with --as-needed, so unused libraries are dropped.
            println!("cargo:rustc-link-lib=dylib=ortools");
            for entry in fs::read_dir(root.join("lib"))? {
                let name = entry?.file_name();
                let Some(lib) = name
                    .to_str()
                    .and_then(|name| name.strip_prefix("lib"))
                    .and_then(|name| name.strip_suffix(".so"))
                else {
                    continue;
                };
                if lib == "protobuf" || lib.starts_with("absl_") || lib.starts_with("utf8_") {
                    println!("cargo:rustc-link-lib=dylib={lib}");
                }
            }
        }
    }
    Ok(())
}
