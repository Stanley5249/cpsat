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

// The AlmaLinux 8.10 build needs only glibc 2.28 and the GCC 8 libstdc++, and
// both stay backward compatible, so it serves Debian 12, Ubuntu 22.04, and
// every newer distribution.
const LINUX: Archive = Archive {
    url: "https://github.com/google/or-tools/releases/download/v9.15/or-tools_x86_64_AlmaLinux-8.10_cpp_v9.15.6755.tar.gz",
    sha256: "c43b473b0d141cee253659e8e9e9f46255b7a5edd3839abe7bf4c42a7910a1e9",
};

pub fn prebuilt(target: &str) -> Option<&'static Archive> {
    match target {
        "x86_64-pc-windows-msvc" => Some(&WINDOWS),
        "x86_64-unknown-linux-gnu" => Some(&LINUX),
        _ => None,
    }
}
