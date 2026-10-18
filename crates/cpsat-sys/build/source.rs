//! Builds OR-Tools from the pinned source with upstream CMake.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Result;
use crate::fetch::run;

/// Options turned off because CP-SAT does not need them. Defaults verified in
/// the v9.15 root `CMakeLists.txt`.
const OPTIONS: &[&str] = &[
    "-DCMAKE_BUILD_TYPE=Release",
    "-DBUILD_SHARED_LIBS=ON",
    "-DBUILD_DEPS=ON",
    "-DBUILD_CXX=ON",
    "-DBUILD_PYTHON=OFF",
    "-DBUILD_JAVA=OFF",
    "-DBUILD_DOTNET=OFF",
    "-DBUILD_SAMPLES=OFF",
    "-DBUILD_EXAMPLES=OFF",
    "-DBUILD_TESTING=OFF",
    "-DBUILD_MATH_OPT=OFF",
    "-DBUILD_FLATZINC=OFF",
    "-DUSE_SCIP=OFF",
    "-DUSE_HIGHS=OFF",
    "-DUSE_COINOR=OFF",
    "-DUSE_GLPK=OFF",
];

/// Configures, builds, and installs under `dir`, and returns the install
/// prefix. `dir` lives in the shared cache, because a full build takes about an
/// hour and should survive `cargo clean`.
pub fn build(source: &Path, dir: &Path) -> Result<PathBuf> {
    let build = dir.join("build");
    let install = dir.join("install");
    if install.join(".complete").exists() {
        return Ok(install);
    }
    run(Command::new("cmake")
        .arg("-S")
        .arg(source)
        .arg("-B")
        .arg(&build)
        .args(OPTIONS)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install.display())))?;
    run(Command::new("cmake").arg("--build").arg(&build).args([
        "--config",
        "Release",
        "--target",
        "install",
        "--parallel",
    ]))?;
    std::fs::write(install.join(".complete"), "")?;
    Ok(install)
}
