//! Downloads, verifies, and extracts archives into a cache shared across
//! builds, so `cargo clean` does not force a new download.
//!
//! Downloading and extracting shell out to `curl` and `tar`, which ship with
//! Windows 10 and later and with common Linux distributions. They add no
//! build-dependency compile time and use the system proxy and certificate
//! store. Switch to `ureq` with the `tar` and `zip` crates if a supported
//! platform lacks either tool, or to extract only the files the build needs.

use std::env;
use std::fmt::Write;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};

use crate::artifacts::Archive;

pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("CPSAT_CACHE_DIR") {
        let dir = PathBuf::from(dir);
        // The build script runs in the package directory, so a relative path
        // would not mean what the user typed.
        ensure!(
            dir.is_absolute(),
            "CPSAT_CACHE_DIR must be absolute: {}",
            dir.display()
        );
        return Ok(dir);
    }
    default_cache_base()
        .map(|dir| dir.join("cpsat"))
        .context("no cache directory found; set CPSAT_CACHE_DIR")
}

#[cfg(windows)]
fn default_cache_base() -> Option<PathBuf> {
    env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

#[cfg(not(windows))]
fn default_cache_base() -> Option<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
}

/// Fills `dir` once across all builds. A lock file serializes concurrent
/// builds, and `fill` runs again after an interrupted attempt, because only a
/// finished one writes the `.complete` marker.
pub fn once(dir: &Path, fill: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let marker = dir.join(".complete");
    if marker.exists() {
        return Ok(());
    }
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let lock = File::create(dir.with_extension("lock"))?;
    lock.lock()?;
    if !marker.exists() {
        fill(dir)?;
        fs::write(&marker, "")?;
    }
    Ok(())
}

/// Returns the single top-level directory of the extracted archive.
pub fn extracted(archive: &Archive, cache: &Path) -> Result<PathBuf> {
    let key = &archive.sha256[..16];
    let file = download(archive, &cache.join("archives").join(key))?;
    let dir = cache.join("extracted").join(key);
    once(&dir, |dir| {
        run(Command::new(tar()).arg("-xf").arg(&file).arg("-C").arg(dir))
    })?;
    single_subdir(&dir)
}

fn download(archive: &Archive, dir: &Path) -> Result<PathBuf> {
    let name = archive
        .url
        .rsplit('/')
        .next()
        .expect("archive URLs end in a file name");
    let file = dir.join(name);
    once(dir, |_| {
        run(Command::new("curl")
            .args(["--fail", "--location", "--silent", "--show-error"])
            // Refuse a redirect to plain HTTP, even though the checksum is pinned.
            .args(["--proto", "=https", "--proto-redir", "=https"])
            .args(["--retry", "3", "--connect-timeout", "30"])
            // Abort a stalled transfer instead of hanging the build.
            .args(["--speed-limit", "1024", "--speed-time", "60"])
            .arg("--output")
            .arg(&file)
            .arg(archive.url))?;
        let actual = sha256(&file)?;
        if actual != archive.sha256 {
            fs::remove_file(&file)?;
            bail!(
                "checksum mismatch for {}: expected {}, got {actual}",
                archive.url,
                archive.sha256
            );
        }
        Ok(())
    })?;
    Ok(file)
}

// A GNU tar earlier on PATH, such as Git's, cannot read zip files, so use the
// bsdtar that ships with Windows.
#[cfg(windows)]
fn tar() -> PathBuf {
    let root = env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    Path::new(&root).join("System32").join("tar.exe")
}

#[cfg(not(windows))]
fn tar() -> PathBuf {
    PathBuf::from("tar")
}

fn single_subdir(dir: &Path) -> Result<PathBuf> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            dirs.push(entry.path());
        }
    }
    match <[PathBuf; 1]>::try_from(dirs) {
        Ok([only]) => Ok(only),
        Err(_) => bail!("expected one top-level directory in {}", dir.display()),
    }
}

pub fn sha256(file: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    io::copy(&mut File::open(file)?, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut hex, byte| {
        write!(hex, "{byte:02x}").expect("writing to a String never fails");
        hex
    })
}

pub fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to start {command:?}"))?;
    ensure!(status.success(), "{command:?} exited with {status}");
    Ok(())
}
