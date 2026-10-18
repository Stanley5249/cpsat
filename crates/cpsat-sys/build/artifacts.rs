//! Pinned OR-Tools archives.
//!
//! To bump the version, update every URL, then refresh each SHA-256 from a
//! real download, for example `curl -fsSL <url> | sha256sum`, and compare it
//! with the `digest` that `gh api repos/google/or-tools/releases/tags/<tag>`
//! reports for the asset. Keep the vendored protos in `crates/cpsat/proto` on
//! the same tag.

pub struct Archive {
    pub url: &'static str,
    pub sha256: &'static str,
}

pub const SOURCE: Archive = Archive {
    url: "https://github.com/google/or-tools/archive/refs/tags/v9.15.tar.gz",
    sha256: "6395a00a97ff30af878ee8d7fd5ad0ab1c7844f7219182c6d71acbee1b5f3026",
};

const WINDOWS: Archive = Archive {
    url: "https://github.com/google/or-tools/releases/download/v9.15/or-tools_x64_VisualStudio2022_cpp_v9.15.6755.zip",
    sha256: "43429c741641c8b495ee77e44ea00f0f4524519495fd2edaf929003aa2b2ea30",
};

// Ubuntu 24.04 is the oldest supported Linux. glibc and libstdc++ stay backward
// compatible, so this archive also serves newer distributions.
const LINUX: Archive = Archive {
    url: "https://github.com/google/or-tools/releases/download/v9.15/or-tools_amd64_ubuntu-24.04_cpp_v9.15.6755.tar.gz",
    sha256: "6f389320672cee00b78aacefb2bde33fef0bb988c3b2735573b9fffd1047fbda",
};

pub fn prebuilt(target: &str) -> Option<&'static Archive> {
    match target {
        "x86_64-pc-windows-msvc" => Some(&WINDOWS),
        "x86_64-unknown-linux-gnu" => Some(&LINUX),
        _ => None,
    }
}
