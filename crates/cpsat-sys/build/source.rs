//! Builds OR-Tools from the pinned source with upstream CMake.

use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Builds and installs into the shared cache, because a full build is slow
/// and should survive `cargo clean`, and returns the install prefix.
///
/// The directory is keyed by the source, the options, and the target, so a
/// change to any of them builds anew. A failed build keeps its CMake tree, so
/// the next attempt continues incrementally.
pub fn build(cache: &Path, target: &str) -> Result<PathBuf> {
    let source = fetch::extracted(&artifacts::SOURCE, cache)?;
    let mut key = Sha256::new();
    for part in [artifacts::SOURCE.sha256, target].iter().chain(OPTIONS) {
        key.update(part.as_bytes());
        key.update([0]);
    }
    let dir = cache
        .join("source")
        .join(&fetch::hex(&key.finalize())[..16]);
    let install = dir.join("install");
    fetch::once(&dir, |dir| {
        let build = dir.join("build");
        fetch::run(
            Command::new("cmake")
                .arg("-S")
                .arg(&source)
                .arg("-B")
                .arg(&build)
                .args(OPTIONS)
                .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install.display())),
        )?;
        // Multi-config generators such as Visual Studio need --config.
        fetch::run(Command::new("cmake").arg("--build").arg(&build).args([
            "--config",
            "Release",
            "--target",
            "install",
            "--parallel",
        ]))
    })?;
    Ok(install)
}
