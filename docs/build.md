# Build and deployment

How `cpsat-sys` acquires OR-Tools, links it, and how to ship the result. The README lists the features and environment variables.

## Acquisition

The build script picks the first mode that applies:

```text
ORTOOLS_ROOT set?            --yes--> link that install
        | no
feature build-libortools?    --yes--> build from source with CMake
        | no
feature download-libortools? --yes--> download the official release (default)
        | no
error
```

`ORTOOLS_ROOT` is an environment variable because a path is machine-specific and cannot live in `Cargo.toml`. It means what `ortools_ROOT` means to CMake's `find_package`: the install prefix that holds `include` and `lib`. Windows ignores case in variable names, so there the two are one variable with one meaning.

## Offline builds

OR-Tools cannot be vendored inside the crate, as `openssl-src` or `rusqlite`'s `bundled` do. The source archive exceeds the 10 MB crates.io limit, and its CMake build fetches Abseil, Protobuf, and other dependencies during configure. The pinned URL and SHA-256 in `crates/cpsat-sys/build/artifacts.rs` stand in for a vendored copy.

To build without network, do one of these:

- Extract the official archive for the target, and set `ORTOOLS_ROOT` to the directory inside it that holds `include` and `lib`.
- Copy a filled `CPSAT_CACHE_DIR` from a machine with network.

Deleting `CPSAT_CACHE_DIR` makes the next build download and extract again. `ORTOOLS_ROOT` and `CPSAT_CACHE_DIR` must be absolute. `cargo run` and `cargo test` find the OR-Tools shared libraries while Cargo's `build.build-dir` stays inside the target directory, which is the default. The `cpsat-sys` build script explains why.

## Linking

`cpsat-sys` links OR-Tools dynamically. The official archives hold only shared libraries, and the build script rejects an install whose `ortoolsTargets.cmake` declares a static `ortools::ortools`.

Static linking is planned for `build-libortools`, through a `static` feature that passes `-DBUILD_SHARED_LIBS=OFF`. The ship-one-file benefit costs this:

- `link` must emit the static archives in dependency order, read from `INTERFACE_LINK_LIBRARIES`, because GNU ld resolves `.a` files in order.
- A source build takes about an hour, and on Linux it raises the glibc and libstdc++ floor to the build machine's, while the AlmaLinux 8 archive needs only glibc 2.28.
- Another crate that links Protobuf or Abseil statically into the same binary causes duplicate symbols.

Loading OR-Tools at run time, as `ort`'s `load-dynamic` does, would need the C++ bridge itself to become a loaded library, so it is out of scope.

## Deployment

A shipped executable needs the OR-Tools shared libraries beside it. Copy the contents of the runtime directory next to the executable. The directory also holds `licenses`, the notices that OR-Tools and its bundled libraries require you to ship with them.

The runtime directory is the `runtime` folder in the `cpsat-sys` build output. To find it from your own build script, add `cpsat-sys` as a direct dependency and read `DEP_ORTOOLS_RUNTIME`, because Cargo passes it only to packages that depend on `cpsat-sys` directly.

Windows finds DLLs in the executable's directory. Linux needs an `$ORIGIN` rpath, which a library cannot set for your binary, so add it to `.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-Wl,-rpath,$ORIGIN"]
```

This repository's `.cargo/config.toml` does the same, and the CI `test` job runs `crates/cpsat/examples/solve.rs` from a directory that holds only the copied runtime. A `RUSTFLAGS` environment variable replaces these `rustflags`.

`cpsat-sys` leaves the target directory alone. `ort`'s `copy-dylibs` links libraries into `target/<profile>` by guessing that path from `OUT_DIR`, which writes outside `OUT_DIR` and breaks when `build.build-dir` moves.

## Prior art

| Crate         | Download              | Build from source  | Existing install   |
| ------------- | --------------------- | ------------------ | ------------------ |
| `ort-sys`     | `download-binaries`   | none               | `ORT_LIB_LOCATION` |
| `torch-sys`   | `download-libtorch`   | none               | `LIBTORCH`         |
| `openssl-sys` | none                  | `vendored`         | `OPENSSL_DIR`      |
| `scip-sys`    | `bundled`             | `from-source`      | `SCIPOPTDIR`       |
| `highs-sys`   | none                  | `build` (default)  | `discover` feature |
| `cpsat-sys`   | `download-libortools` | `build-libortools` | `ORTOOLS_ROOT`     |

`download-libortools` follows `torch-sys`. `vendored` and `bundled` usually mean source shipped inside the crate, which does not fit here.
