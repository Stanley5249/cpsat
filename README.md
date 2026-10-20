# cpsat

Rust binding for the Google OR-Tools CP-SAT solver.

`Model` builds a model with typed variables, linear expressions, and
constraints, like Python's `CpModel`. It stores the model as a
[Buffa](https://github.com/anthropics/buffa)-generated `CpModelProto`, and
parameters and responses are Buffa types too, so every schema field is
available. `cargo build` downloads the official OR-Tools release and compiles a
small C++ bridge.

## Installation

```toml
[dependencies]
cpsat = { git = "https://github.com/Stanley5249/cpsat" }
```

Supported targets are `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` on
glibc 2.28 or newer, which CI tests on AlmaLinux 8 and Ubuntu 26.04. Building
needs a C++ compiler (MSVC 2022 or GCC 11 or newer), `curl`, and `tar`, plus
CMake for `CPSAT_ORTOOLS=source`.

## Usage

```rust
use std::ops::ControlFlow;

use cpsat::prelude::*;
use cpsat::proto::SatParameters;

let mut model = Model::new();
let x = model.new_var(0..=10);
let y = model.new_var(0..=10);
let enabled = model.new_var(Bool);
model.add((x + 2 * y).le(14));
model.add(x.ne(y).only_enforce_if(enabled));
model.add(Constraint::all_different([x, y]));
model.maximize(x + y);

let response = model.solve(&SatParameters::default())?;
println!("{}: x = {:?}", response.status(), response.value(x));

let stop = StopHandle::new();
let response = Solver::new(&SatParameters::default().with_num_workers(8))
    .on_event(EventSet::SOLUTION, |event| {
        if let Event::Solution(view) = event {
            println!("objective {}", view.objective_value);
        }
        ControlFlow::Continue(())
    })
    .with_stop_handle(&stop)
    .solve(&model)?;
```

`crates/cpsat/tests/e2e/samples.rs` ports the official OR-Tools samples, and
`docs/parity.md` maps each Python method to its Rust counterpart.
`Model::proto_mut` reaches any schema field that the modeling API does not
cover.

## Configuration

The build script reads these environment variables:

- `CPSAT_ORTOOLS`: `prebuilt` (default), `source` to build OR-Tools with CMake,
  or `dir:<path>`
- `CPSAT_CACHE_DIR`: Download and build cache, by default
  `%LOCALAPPDATA%\cpsat`, or `$XDG_CACHE_HOME/cpsat` then `~/.cache/cpsat`

For an offline build, extract the official OR-Tools archive for the target and
set `CPSAT_ORTOOLS=dir:<path>` to the extracted directory that holds `include`
and `lib`. `crates/cpsat-sys/build/artifacts.rs` lists each archive's URL and
SHA-256.

Deleting `CPSAT_CACHE_DIR` makes the next build download and extract again. The
`dir:` path and `CPSAT_CACHE_DIR` must be absolute. `cargo run` and `cargo test`
find the OR-Tools shared libraries while Cargo's `build.build-dir` stays inside
the target directory, which is the default. The `cpsat-sys` build script
explains why.

## Deployment

The OR-Tools libraries are shared, so a shipped executable needs them beside it.
Copy the contents of the OR-Tools runtime directory next to the executable. The
directory also holds `licenses`, the notices that OR-Tools and its bundled
libraries require you to ship with them.

The runtime directory is the `runtime` folder in the `cpsat-sys` build output.
To find it from your own build script, add `cpsat-sys` as a direct dependency
and read `DEP_ORTOOLS_RUNTIME`, because Cargo passes it only to packages that
depend on `cpsat-sys` directly.

Windows finds DLLs in the executable's directory. Linux needs an `$ORIGIN`
rpath, which a library cannot set for your binary, so add it to
`.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-Wl,-rpath,$ORIGIN"]
```

This repository's `.cargo/config.toml` does the same, and the CI `test` job runs
`crates/cpsat/examples/solve.rs` from a directory that holds only the copied
runtime. A `RUSTFLAGS` environment variable replaces these `rustflags`.

## Requirements

For development, also install:

- [Rust](https://rustup.rs/)
- [just](https://github.com/casey/just)
- [tombi](https://tombi-toml.github.io/tombi/)
- [Bun](https://bun.sh/) for `bunx oxfmt`
- [Buf](https://buf.build/docs/cli/installation/), only for `just gen-proto` and
  `just check-proto`
- [uv](https://docs.astral.sh/uv/), only for `just gen-fixtures`, which
  regenerates the Python models in `crates/cpsat/tests/e2e/fixtures`

## Commands

Run `just` to list the recipes. `just check` is the fast local gate, and
`just ci` adds the tests and docs.

## Documents

- [Design](docs/design.md)
- [Conventions](docs/conventions.md)
- [Roadmap](docs/roadmap.md)
- [Python parity](docs/parity.md)
- [Proto2 defaults in Buffa](docs/buffa-proto2-defaults.md)

## License

Licensed under either the [Apache License 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. OR-Tools and the vendored `.proto`
files are Apache-2.0.
