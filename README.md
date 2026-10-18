# cpsat

Rust binding for the Google OR-Tools CP-SAT solver, on raw Protobuf models.

Models, parameters, and responses are
[Buffa](https://github.com/anthropics/buffa)-generated types of the OR-Tools
schemas, so every field is available. `cargo build` downloads the official
OR-Tools release and compiles a small C++ bridge.

## Installation

```toml
[dependencies]
cpsat = { git = "https://github.com/Stanley5249/cpsat" }
```

Supported targets are `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` on
Ubuntu 24.04 or newer. Building needs a C++ compiler (MSVC 2022 or GCC 13 or
newer), `curl`, and `tar`.

## Usage

```rust
use std::ops::ControlFlow;

use cpsat::proto::{CpModelProto, SatParameters};
use cpsat::{Event, Events, StopHandle};

let model = CpModelProto::default();
let params = SatParameters { num_workers: Some(8), ..Default::default() };
let response = cpsat::solve(&model, &params)?;

let stop = StopHandle::new();
let response = cpsat::solve_with(&model, &params, Events::SOLUTION, Some(&stop), |event| {
    if let Event::Solution(view) = event {
        println!("objective {}", view.objective_value);
    }
    ControlFlow::Continue(())
})?;
```

## Configuration

The build script reads these environment variables:

| Variable          | Effect                                                                          |
| ----------------- | ------------------------------------------------------------------------------- |
| `CPSAT_ORTOOLS`   | `prebuilt` (default), `source` to build OR-Tools with CMake, or `dir:<path>`    |
| `CPSAT_CACHE_DIR` | Download and build cache, by default `%LOCALAPPDATA%\cpsat` or `~/.cache/cpsat` |

`cargo run` and `cargo test` find the OR-Tools shared libraries automatically. A
shipped executable needs them beside it or on the library path. A build script
that depends on `cpsat-sys` can copy them from `DEP_ORTOOLS_RUNTIME`.

## Requirements

For development, also install:

- [Rust](https://rustup.rs/)
- [just](https://github.com/casey/just)
- [tombi](https://tombi-toml.github.io/tombi/)
- [Bun](https://bun.sh/) for `bunx oxfmt`
- [Buf](https://buf.build/docs/cli/installation/), only for `just gen`

## Commands

Run `just` to list the recipes. `just check` is the fast local gate, and
`just ci` adds the tests and docs.

## Documents

- [Plan](docs/plan.md)
- [Python parity](docs/parity.md)

## License

Licensed under either the [Apache License 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. OR-Tools and the vendored `.proto`
files are Apache-2.0.
