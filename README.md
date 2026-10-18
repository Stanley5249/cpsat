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
glibc 2.28 or newer, such as Debian 12 or Ubuntu 22.04. Building needs a C++
compiler (MSVC 2022 or GCC 11 or newer), `curl`, and `tar`, plus CMake for
`CPSAT_ORTOOLS=source`.

## Usage

```rust
use std::ops::ControlFlow;

use cpsat::prelude::*;

let mut model = Model::new();
let x = model.int_var(0..=10);
let y = model.int_var(0..=10);
let enabled = model.bool_var();
model.add((x + 2 * y).le(14));
model.add(x.ne(y).only_if(enabled));
model.add(Constraint::all_different([x, y]));
model.maximize(x + y);

let response = solve(model.proto(), &SatParameters::default())?;
println!("{}: x = {:?}", response.status(), response.value(x));

let stop = StopHandle::new();
let response = Solver::new(SatParameters::default().with_num_workers(8))
    .on_event(EventSet::SOLUTION, |event| {
        if let Event::Solution(view) = event {
            println!("objective {}", view.objective_value);
        }
        ControlFlow::Continue(())
    })
    .with_stop_handle(&stop)
    .solve(model.proto())?;
```

`crates/cpsat/tests/samples.rs` ports the official OR-Tools samples, and
`docs/parity.md` maps each Python method to its Rust counterpart.
`Model::proto_mut` reaches any schema field that the modeling API does not
cover.

## Configuration

The build script reads these environment variables:

| Variable          | Effect                                                                                                        |
| ----------------- | ------------------------------------------------------------------------------------------------------------- |
| `CPSAT_ORTOOLS`   | `prebuilt` (default), `source` to build OR-Tools with CMake, or `dir:<path>`                                  |
| `CPSAT_CACHE_DIR` | Download and build cache, by default `%LOCALAPPDATA%\cpsat`, or `$XDG_CACHE_HOME/cpsat` then `~/.cache/cpsat` |

Both paths must be absolute. `cargo run` and `cargo test` find the OR-Tools
shared libraries while Cargo's `build.build-dir` stays inside the target
directory, which is the default. The `cpsat-sys` build script explains why. A
shipped executable needs the libraries beside it or on the library path. A build
script that depends on `cpsat-sys` can copy them from `DEP_ORTOOLS_RUNTIME`.

## Requirements

For development, also install:

- [Rust](https://rustup.rs/)
- [just](https://github.com/casey/just)
- [tombi](https://tombi-toml.github.io/tombi/)
- [Bun](https://bun.sh/) for `bunx oxfmt`
- [Buf](https://buf.build/docs/cli/installation/), only for `just proto-gen` and
  `just proto-check`
- [uv](https://docs.astral.sh/uv/), only for `just fixtures`, which regenerates
  the Python models in `crates/cpsat/tests/fixtures`

## Commands

Run `just` to list the recipes. `just check` is the fast local gate, and
`just ci` adds the tests and docs.

## Documents

- [Plan](docs/plan.md)
- [Python parity](docs/parity.md)
- [Draft Buffa issue on proto2 defaults](docs/buffa-proto2-defaults.md)

## License

Licensed under either the [Apache License 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. OR-Tools and the vendored `.proto`
files are Apache-2.0.
