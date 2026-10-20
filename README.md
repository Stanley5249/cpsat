# cpsat

Rust binding for the Google OR-Tools CP-SAT solver.

`CpModel` builds a model with typed variables, linear expressions, and
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
glibc 2.28 or newer, which CI tests on Windows, AlmaLinux 8, and Ubuntu 26.04.
Building needs Rust at the `rust-version` in `Cargo.toml` or newer, a C++
compiler (MSVC 2022 or GCC 11 or newer), `curl`, and `tar`, plus CMake for the
`build-libortools` feature.

## Usage

```rust no_run
use std::ops::ControlFlow;

use cpsat::prelude::*;
use cpsat::proto::SatParameters;

// From `solve_and_print_intermediate_solutions_sample_sat.py`.
fn main() -> cpsat::Result<()> {
    let mut model = CpModel::new();
    let x = model.new_var((0..=2).with_name("x"));
    let y = model.new_var((0..=2).with_name("y"));
    let z = model.new_var((0..=2).with_name("z"));
    model.add(x.ne(y));
    model.maximize(x + 2 * y + 3 * z);

    let response = model.solve(&SatParameters::default())?;
    if let Some(solution) = response.solution() {
        println!("{}: x = {}", response.status(), solution.value(x));
    }

    let stop = StopHandle::new();
    let response = CpSolver::new(&SatParameters::default().with_num_workers(8))
        .add_solution_callback(|response| {
            println!("objective {}", response.objective_value());
            ControlFlow::Continue(())
        })
        .with_stop_handle(&stop)
        .solve(&model)?;
    println!("{}", response.status());
    Ok(())
}
```

`crates/cpsat/tests/e2e/samples.rs` ports the official OR-Tools samples, and
`docs/parity.md` maps each Python method to its Rust counterpart.
`CpModel::proto_mut` reaches any schema field that the modeling API does not
cover.

## Configuration

These features choose where OR-Tools comes from, and
`crates/cpsat-sys/build/artifacts.rs` pins its version:

- `download-libortools` (default): download the official release
- `build-libortools`: build OR-Tools from source with CMake, which wins over
  `download-libortools`

These features choose the message formats:

- `text` (default): Protobuf text format through `buffa::text`
- `views` (default): zero-copy views of encoded messages through `buffa::view`
- `json`: Proto3 JSON through serde

The build script reads these environment variables:

- `ORTOOLS_ROOT`: an existing OR-Tools install, which wins over both features
- `CPSAT_CACHE_DIR`: Download and build cache, by default
  `%LOCALAPPDATA%\cpsat`, or `$XDG_CACHE_HOME/cpsat` then `~/.cache/cpsat`

`docs/build.md` covers the mode order, offline builds, linking, and how to ship
the OR-Tools runtime libraries beside your executable.

## Requirements

For development, also install:

- [just](https://github.com/casey/just), and on Windows
  [PowerShell 7](https://learn.microsoft.com/powershell/scripting/install/installing-powershell-on-windows)
  for its recipes
- [tombi](https://tombi-toml.github.io/tombi/)
- [Bun](https://bun.sh/) for `bunx oxfmt`
- [Buf](https://buf.build/docs/cli/installation/), only for `just gen-proto` and
  `just check-proto`
- [uv](https://docs.astral.sh/uv/), only for `just gen-fixtures`, which
  regenerates the Python models in `crates/cpsat/tests/e2e/fixtures`

## Commands

Run `just` to list the recipes. The contributing guide says which to run before
a commit.

## Documents

- [Contributing](https://github.com/Stanley5249/cpsat/blob/main/CONTRIBUTING.md)
- [Design](https://github.com/Stanley5249/cpsat/blob/main/docs/design.md)
- [Build and deployment](https://github.com/Stanley5249/cpsat/blob/main/docs/build.md)
- [Maintenance](https://github.com/Stanley5249/cpsat/blob/main/docs/maintenance.md)
- [Conventions](https://github.com/Stanley5249/cpsat/blob/main/docs/conventions.md)
- [Roadmap](https://github.com/Stanley5249/cpsat/blob/main/docs/roadmap.md)
- [Python parity](https://github.com/Stanley5249/cpsat/blob/main/docs/parity.md)
- [Proto2 defaults in Buffa](https://github.com/Stanley5249/cpsat/blob/main/docs/buffa-proto2-defaults.md)

## License

Licensed under either the [Apache License 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. OR-Tools and the vendored `.proto`
files are Apache-2.0.
