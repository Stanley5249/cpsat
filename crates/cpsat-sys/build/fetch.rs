//! Downloads, verifies, and extracts archives into a cache shared across
//! builds, so `cargo clean` does not force a new download.
//!
//! Downloading and extracting shell out to `curl` and `tar`, which ship with
//! Windows 10 and later and with common Linux distributions. `tar` also
//! preserves the symbolic links in the Linux archives.

use std::env;
use std::fmt::Write;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use sha2::{Digest, Sha256};

use crate::Result;
use crate::artifacts::Archive;

pub fn cache_dir() -> Result<PathBuf> {
    if let Some(dir) = env::var_os("CPSAT_CACHE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let base = if cfg!(windows) {
        env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
    };
    base.map(|dir| dir.join("cpsat"))
        .context("no cache directory found; set CPSAT_CACHE_DIR")
}

/// Returns the single top-level directory of the extracted archive.
pub fn extracted(archive: &Archive, cache: &Path) -> Result<PathBuf> {
    let dir = cache.join("extracted").join(&archive.sha256[..16]);
    if !dir.join(".complete").exists() {
        let file = download(archive, cache)?;
        extract(&file, &dir)?;
    }
    single_subdir(&dir)
}

fn download(archive: &Archive, cache: &Path) -> Result<PathBuf> {
    let name = archive
        .url
        .rsplit('/')
        .next()
        .expect("archive URLs end in a file name");
    let file = cache
        .join("archives")
        .join(format!("{}-{name}", &archive.sha256[..16]));
    if file.exists() && sha256(&file)? == archive.sha256 {
        return Ok(file);
    }
    fs::create_dir_all(file.parent().expect("archive path has a parent"))?;
    let partial = file.with_extension(format!("{}.part", std::process::id()));
    run(Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--retry",
            "3",
        ])
        .arg("--output")
        .arg(&partial)
        .arg(archive.url))?;
    let actual = sha256(&partial)?;
    if actual != archive.sha256 {
        fs::remove_file(&partial)?;
        bail!(
            "checksum mismatch for {}: expected {}, got {actual}",
            archive.url,
            archive.sha256
        );
    }
    fs::rename(&partial, &file)?;
    Ok(file)
}

fn extract(file: &Path, dir: &Path) -> Result<()> {
    // Extract beside the final directory and rename it into place, so a
    // concurrent or interrupted build never sees a partial tree.
    let partial = dir.with_extension(format!("{}.part", std::process::id()));
    if partial.exists() {
        fs::remove_dir_all(&partial)?;
    }
    fs::create_dir_all(&partial)?;
    run(Command::new(tar())
        .arg("-xf")
        .arg(file)
        .arg("-C")
        .arg(&partial))?;
    fs::write(partial.join(".complete"), "")?;
    if dir.exists() {
        // Another build finished first, so its tree is equivalent.
        fs::remove_dir_all(&partial)?;
        return Ok(());
    }
    fs::rename(&partial, dir)?;
    Ok(())
}

// On Windows, a GNU tar earlier on PATH, such as Git's, cannot read zip files,
// so use the bsdtar that ships with the system.
fn tar() -> PathBuf {
    match env::var_os("SystemRoot") {
        Some(root) if cfg!(windows) => Path::new(&root).join("System32").join("tar.exe"),
        _ => PathBuf::from("tar"),
    }
}

fn single_subdir(dir: &Path) -> Result<PathBuf> {
    let mut dirs = fs::read_dir(dir)?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.path().is_dir());
    match (dirs.next(), dirs.next()) {
        (Some(only), None) => Ok(only.path()),
        _ => bail!("expected one top-level directory in {}", dir.display()),
    }
}

fn sha256(file: &Path) -> Result<String> {
    let mut reader = fs::File::open(file)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 20];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to a String never fails");
            hex
        }))
}

pub fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to start {command:?}"))?;
    if !status.success() {
        bail!("{command:?} exited with {status}");
    }
    Ok(())
}
