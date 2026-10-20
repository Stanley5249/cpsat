//! Acquires OR-Tools, compiles the C ABI bridge, and emits link directives.
//!
//! `ORTOOLS_ROOT` names an existing install. Otherwise the features choose:
//! `build-libortools` builds from source, and `download-libortools` (default)
//! downloads the release. Features are additive, so the explicit choice wins
//! over the default.
//!
//! The shared libraries are linked or copied under `OUT_DIR`, because Cargo
//! puts native search paths inside the target directory on `PATH` or
//! `LD_LIBRARY_PATH` for `cargo run` and `cargo test`. Cargo skips paths
//! outside it, so this fails when `build.build-dir` leaves the target
//! directory.

#[path = "build/artifacts.rs"]
mod artifacts;
#[path = "build/fetch.rs"]
mod fetch;
#[path = "build/source.rs"]
mod source;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

enum Mode {
    Download,
    Build,
    Root(PathBuf),
}

enum Toolchain {
    Msvc,
    Gnu,
}

/// Files the build reads from an OR-Tools install. The CMake file lives in the
/// library directory, which is `lib64` in the AlmaLinux archive.
const HEADER: &str = "include/ortools/sat/swig_helper.h";
const CMAKE_TARGETS: &str = "cmake/ortools/ortoolsTargets.cmake";
const LIB_DIRS: [&str; 2] = ["lib", "lib64"];

fn main() -> Result<()> {
    for path in ["build.rs", "build", "bridge"] {
        println!("cargo::rerun-if-changed={path}");
    }
    // The default cache directory comes from the platform variables.
    for var in [
        "ORTOOLS_ROOT",
        "CPSAT_CACHE_DIR",
        "DOCS_RS",
        "LOCALAPPDATA",
        "XDG_CACHE_HOME",
        "HOME",
    ] {
        println!("cargo::rerun-if-env-changed={var}");
    }
    // docs.rs builds without network, and rustdoc never links.
    if env::var_os("DOCS_RS").is_some() {
        return Ok(());
    }

    let target = env::var("TARGET")?;
    let out = PathBuf::from(env::var("OUT_DIR")?);
    let toolchain = toolchain(&target)?;
    let root = match mode()? {
        Mode::Download => {
            let archive = artifacts::prebuilt(&target).with_context(|| {
                format!(
                    "no prebuilt OR-Tools for {target}; enable the build-libortools \
                     feature or set ORTOOLS_ROOT"
                )
            })?;
            fetch::extracted(archive, &fetch::cache_dir()?)?
        }
        Mode::Build => source::build(&fetch::cache_dir()?, &target)?,
        Mode::Root(root) => {
            println!("cargo::rerun-if-changed={}", root.join(HEADER).display());
            root
        }
    };
    ensure!(
        root.join(HEADER).exists(),
        "{} is not an OR-Tools C++ install",
        root.display()
    );
    let lib = LIB_DIRS
        .iter()
        .map(|dir| root.join(dir))
        .find(|lib| lib.join(CMAKE_TARGETS).exists())
        .with_context(|| {
            format!(
                "no lib/{CMAKE_TARGETS} or lib64/{CMAKE_TARGETS} in {}",
                root.display()
            )
        })?;
    let targets = lib.join(CMAKE_TARGETS);
    println!("cargo::rerun-if-changed={}", targets.display());
    let targets = fs::read_to_string(&targets)
        .with_context(|| format!("failed to read {}", targets.display()))?;
    // A static install needs its archives linked in dependency order, which
    // `link` does not do yet.
    ensure!(
        !targets.contains("add_library(ortools::ortools STATIC IMPORTED"),
        "{} is a static OR-Tools install; static linking is not supported yet",
        root.display()
    );

    let runtime = out.join("runtime");
    place_runtime(&root, &lib, &runtime, &toolchain)?;
    compile_bridge(&root, &targets, &toolchain)?;
    link(&lib, &runtime, &toolchain)?;

    // Dependents read these as DEP_ORTOOLS_ROOT and DEP_ORTOOLS_RUNTIME, for
    // example to copy the runtime libraries next to a shipped executable.
    println!("cargo::metadata=root={}", root.display());
    println!("cargo::metadata=runtime={}", runtime.display());
    Ok(())
}

fn mode() -> Result<Mode> {
    if let Some(path) = env::var_os("ORTOOLS_ROOT") {
        let path = PathBuf::from(path);
        // The build script runs in the package directory, so a relative path
        // would not mean what the user typed.
        ensure!(
            path.is_absolute(),
            "ORTOOLS_ROOT must be absolute: {}",
            path.display()
        );
        return Ok(Mode::Root(path));
    }
    if env::var_os("CARGO_FEATURE_BUILD_LIBORTOOLS").is_some() {
        return Ok(Mode::Build);
    }
    if env::var_os("CARGO_FEATURE_DOWNLOAD_LIBORTOOLS").is_some() {
        return Ok(Mode::Download);
    }
    bail!("no OR-Tools source; enable download-libortools or build-libortools, or set ORTOOLS_ROOT")
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

fn place_runtime(root: &Path, lib: &Path, runtime: &Path, toolchain: &Toolchain) -> Result<()> {
    if runtime.exists() {
        fs::remove_dir_all(runtime)?;
    }
    fs::create_dir_all(runtime)?;
    let (dir, wanted): (_, fn(&str) -> bool) = match toolchain {
        Toolchain::Msvc => (root.join("bin"), |name| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
        }),
        Toolchain::Gnu => (lib.to_owned(), |name| name.contains(".so")),
    };
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_str().is_some_and(wanted) {
            place(&entry.path(), &runtime.join(&name))?;
        }
    }
    // Apache-2.0 requires the notices to travel with redistributed libraries,
    // so they sit where users copy the libraries from. A `dir:` install may
    // lack them, which leaves the notices to its owner.
    for notices in ["share/doc/ortools", "share/licenses"] {
        let from = root.join(notices);
        if from.is_dir() {
            place_tree(&from, &runtime.join("licenses"))?;
        }
    }
    Ok(())
}

fn place_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("failed to read {}", from.display()))? {
        let entry = entry?;
        let to = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            place_tree(&entry.path(), &to)?;
        } else {
            place(&entry.path(), &to)?;
        }
    }
    Ok(())
}

/// Hard-links one library, because every `OUT_DIR` would otherwise hold its
/// own 90 MB copy, and copies it when the link fails across volumes.
fn place(from: &Path, to: &Path) -> Result<()> {
    // Keep the soname symlink chain that the dynamic loader follows.
    #[cfg(unix)]
    if from.is_symlink() {
        std::os::unix::fs::symlink(fs::read_link(from)?, to)?;
        return Ok(());
    }
    if fs::hard_link(from, to).is_err() {
        fs::copy(from, to)?;
    }
    Ok(())
}

fn compile_bridge(root: &Path, targets: &str, toolchain: &Toolchain) -> Result<()> {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("bridge/bridge.cc")
        .include("bridge")
        .include(root.join("include"));
    // Match the definitions OR-Tools exports to its CMake consumers, because
    // some of them change declarations in the installed headers.
    for define in exported_definitions(targets)? {
        match define.split_once('=') {
            Some((name, value)) => build.define(name, value),
            None => build.define(&define, None),
        };
    }
    // The rest mirror the exported compile options and the Protobuf and Abseil
    // DLL definitions, which the bridge needs to import their symbols.
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
fn exported_definitions(targets: &str) -> Result<Vec<String>> {
    let block = targets
        .split("set_target_properties(ortools::ortools PROPERTIES")
        .nth(1)
        .and_then(|rest| rest.split("\n)").next())
        .with_context(|| format!("no ortools::ortools target in {CMAKE_TARGETS}"))?;
    let definitions = block
        .lines()
        .find_map(|line| line.trim().strip_prefix("INTERFACE_COMPILE_DEFINITIONS "))
        .with_context(|| format!("no compile definitions in {CMAKE_TARGETS}"))?;
    // The pinned archives export no generator expressions here. Dropping one
    // that a future version adds could change the bridge's ABI silently.
    ensure!(
        !definitions.contains("$<"),
        "generator expression in the compile definitions of {CMAKE_TARGETS}"
    );
    Ok(definitions
        .trim_matches('"')
        .split(';')
        .filter(|define| !define.is_empty())
        .map(str::to_owned)
        .collect())
}

fn link(lib: &Path, runtime: &Path, toolchain: &Toolchain) -> Result<()> {
    println!("cargo::rustc-link-search=native={}", runtime.display());
    match toolchain {
        Toolchain::Msvc => {
            println!("cargo::rustc-link-search=native={}", lib.display());
            for lib in ["ortools", "libprotobuf", "abseil_dll"] {
                println!("cargo::rustc-link-lib=dylib={lib}");
            }
        }
        Toolchain::Gnu => {
            // The bridge calls Protobuf and Abseil directly, and GNU ld rejects
            // symbols that resolve only through another shared library. rustc
            // links with --as-needed, so unused libraries are dropped.
            println!("cargo::rustc-link-lib=dylib=ortools");
            for entry in fs::read_dir(lib)? {
                let name = entry?.file_name();
                let Some(lib) = name
                    .to_str()
                    .and_then(|name| name.strip_prefix("lib"))
                    .and_then(|name| name.strip_suffix(".so"))
                else {
                    continue;
                };
                if lib == "protobuf" || lib.starts_with("absl_") || lib.starts_with("utf8_") {
                    println!("cargo::rustc-link-lib=dylib={lib}");
                }
            }
        }
    }
    Ok(())
}
