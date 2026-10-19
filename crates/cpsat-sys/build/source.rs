//! Builds OR-Tools from the pinned source with upstream CMake.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::artifacts;
use crate::fetch;

/// Options that differ from the v9.15 defaults in the root `CMakeLists.txt`.
/// `BUILD_MATH_OPT` stays on, because `ortools_gurobi` links
/// `ortools::math_opt_proto` and fails to generate without it. The install
/// directories are fixed because `GNUInstallDirs` may pick `lib64`.
const OPTIONS: &[&str] = &[
    "-DCMAKE_INSTALL_LIBDIR=lib",
    "-DCMAKE_INSTALL_BINDIR=bin",
    "-DBUILD_DEPS=ON",
    "-DBUILD_SAMPLES=OFF",
    "-DBUILD_EXAMPLES=OFF",
    "-DBUILD_TESTING=OFF",
    "-DBUILD_FLATZINC=OFF",
    "-DUSE_SCIP=OFF",
    "-DUSE_HIGHS=OFF",
    "-DUSE_COINOR=OFF",
];

/// The variables that choose CMake's compiler, flags, and generator, which an
/// installation built under other values may not be ABI compatible with.
const TOOLCHAIN: &[&str] = &[
    "CC",
    "CXX",
    "CFLAGS",
    "CXXFLAGS",
    "LDFLAGS",
    "CMAKE_GENERATOR",
    "CMAKE_TOOLCHAIN_FILE",
];

/// Builds and installs into the shared cache, because a full build is slow
/// and should survive `cargo clean`, and returns the install prefix.
///
/// The directory is keyed by the source, the options, the target, and the
/// set [`TOOLCHAIN`] variables, so a change to any of them builds anew. A
/// failed build keeps its CMake tree, so the next attempt continues
/// incrementally.
pub fn build(cache: &Path, target: &str) -> Result<PathBuf> {
    let source = fetch::extracted(&artifacts::SOURCE, cache)?;
    let mut key = Sha256::new();
    for part in [artifacts::SOURCE.sha256, target].iter().chain(OPTIONS) {
        key.update(part.as_bytes());
        key.update([0]);
    }
    // Only set variables enter the key, so the default environment keeps the
    // directory that builds before this key existed.
    for var in TOOLCHAIN {
        println!("cargo::rerun-if-env-changed={var}");
        if let Some(value) = env::var_os(var) {
            for part in [var.as_bytes(), value.as_encoded_bytes()] {
                key.update(part);
                key.update([0]);
            }
        }
    }
    let dir = cache
        .join("source")
        .join(&fetch::hex(&key.finalize())[..16]);
    let install = dir.join("install");
    let jobs = jobs();
    fetch::once(&dir, |dir| {
        let build = dir.join("build");
        fetch::run(
            cmake(jobs)
                .arg("-S")
                .arg(&source)
                .arg("-B")
                .arg(&build)
                .args(OPTIONS)
                .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install.display())),
        )?;
        // Multi-config generators such as Visual Studio need --config.
        fetch::run(
            cmake(jobs)
                .arg("--build")
                .arg(&build)
                .args(["--config", "Release", "--target", "install"])
                .arg(format!("--parallel={jobs}")),
        )
    })?;
    Ok(install)
}

/// The parallel build jobs: `CMAKE_BUILD_PARALLEL_LEVEL` when set, otherwise
/// three quarters of the hardware threads, so a build of most of an hour
/// leaves the machine usable.
fn jobs() -> usize {
    env::var("CMAKE_BUILD_PARALLEL_LEVEL")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| {
            let threads = thread::available_parallelism().map_or(1, usize::from);
            (threads * 3 / 4).max(1)
        })
}

fn cmake(jobs: usize) -> Command {
    let mut command = Command::new("cmake");
    // Dependencies that configure builds through FetchContent read the level
    // from the environment.
    command.env("CMAKE_BUILD_PARALLEL_LEVEL", jobs.to_string());
    // OR-Tools adds /MP, which lets every MSBuild project spawn one compiler
    // per thread on top of --parallel. /MP1 last on the command line keeps
    // the job count the only limit.
    if cfg!(windows) {
        command.env("_CL_", "/MP1");
    }
    command
}
