# Rust CP-SAT binding plan

Rust owns the model. Protobuf defines the contract. C++ executes the solver. The
FFI exposes only capabilities that Rust cannot implement directly.

## Objectives

Build a Rust-native binding for Google OR-Tools CP-SAT with at least the
functional capabilities of the official Python CP-SAT binding, without
reproducing its high-level API.

### Hard requirements

- The platforms that the README's Installation section lists.
- `cargo build` automatically acquires OR-Tools and builds the native bridge.
- No manually installed OR-Tools, Conda, or Python runtime.
- Official prebuilt OR-Tools archives by default, a reproducible source build as
  an opt-in, and a local directory as a third mode.
- Buffa-generated Rust Protobuf types as the primary model representation, with
  every `CpModelProto`, `SatParameters`, and `CpSolverResponse` field exposed.
- Solution, best-bound, and log callbacks.
- Cancellation from inside a callback and from another thread.
- Python CP-SAT model manipulation and solver-control capabilities.
- Minimal unsafe Rust, C++ code, native dependencies, and conversions.
- A layout ready for crates.io publication later.

### Non-goals

- Python's modeling DSL, `CpModelBuilder`, or the C++ class hierarchy. Small
  helpers on the generated types cover the common cases instead.
- A persistent incremental solver.
- OR-Tools subsystems outside CP-SAT.
- Custom propagation or search algorithms.
- A CP-SAT-only source fork.
- `Domain` arithmetic and text-format parameter or model I/O. Both can be added
  later through `CpSatHelper` if needed.

## Facts verified against v9.15

These facts shape the design. Recheck them when the pinned version changes.

- `ortools/sat/CMakeLists.txt` compiles every `ortools/sat/*.cc` into the
  `ortools` library, so the prebuilt library already contains `SolveWrapper` and
  `CpSatHelper` from `swig_helper.h`, the native core of Python's solver.
  `SolveWrapper` serves one solve.
- The C API in `cp_solver_c.h` has no callbacks, and it `CHECK`s on a parse
  failure, which aborts the process. The bridge therefore wraps `SolveWrapper`
  instead.
- CP-SAT calls solution and best-bound callbacks under its response mutex, and
  `SolverLogger` says CP-SAT never calls it concurrently. Log and solution
  events can still come from different threads, so the bridge serializes them.
- Python's `ExtSolveWrapper` stores the first callback exception, calls
  `StopSearch()`, and rethrows after `Solve` returns.
- Both prebuilt archives are dynamic. `ortools.dll` loads HiGHS and SCIP even
  for CP-SAT. `ortoolsTargets.cmake` exports the compile definitions consumers
  need.
- `cp_model.proto` is proto3 and `sat_parameters.proto` is proto2.

## Architecture

```text
user code
   |
crates/cpsat          Buffa types, helpers, Solver, StopHandle, diagnostics
   |  (safe Rust, owns the model)
crates/cpsat-sys      build.rs, raw extern "C" declarations, links = "ortools"
   |  (serialized bytes, fn pointer + context, opaque session)
native/bridge.cc      extern "C" over SolveWrapper and CpSatHelper
   |
ortools.dll / libortools.so  (SolveCpModel)
```

Rust owns the model and parameter representation, serialization, every model
change, callback handling, and error interpretation. C++ owns solver execution,
callback registration, the active solve and its stop mechanism, and native
diagnostics. A native session lives for one solve and never holds a second copy
of the model.

## Decisions

| Topic             | Decision                                                                                             | Reason                                                                                                                     |
| ----------------- | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Crates            | `cpsat-sys` and `cpsat` under `crates/`                                                              | Cargo's `links` key allows one native link per graph                                                                       |
| Codegen           | `just proto-gen` runs `buf generate` with the remote Buffa plugin, and the output is committed       | Buffa's recommended path. Normal builds need no `buf` or `protoc`                                                          |
| Plugin pin        | `buf.gen.yaml` pins the plugin to the runtime version in `Cargo.toml`                                | Generated code must match the runtime                                                                                      |
| Bridge build      | `cc` crate with definitions read from `ortoolsTargets.cmake`                                         | Prebuilt mode needs only a C++ compiler, not CMake                                                                         |
| Download          | `curl` and `tar` from the system                                                                     | No build-dependency compile time, and the system proxy and CA store apply. `build/fetch.rs` names when to switch to `ureq` |
| Cache             | Archives, extractions, and source builds in `CPSAT_CACHE_DIR`, each filled once under a `File::lock` | Survives `cargo clean`, one extraction serves every `OUT_DIR`, and concurrent builds wait instead of racing                |
| MSRV              | 1.89                                                                                                 | `File::lock`                                                                                                               |
| Runtime libraries | Hard-linked, or copied across volumes, into `OUT_DIR/runtime`                                        | See the `cpsat-sys` build script                                                                                           |
| Errors            | `thiserror` enum in `cpsat`, `anyhow` in the build script                                            | Callers can match an invalid model or a native failure, and the build script reports a context chain                       |
| Solve API         | `cpsat::solve` for the plain case, and a `Solver` that owns its parameters for events and stops      | Mirrors Python's `CpSolver` without a mandatory object for the plain case                                                  |
| Events            | One `FnMut(Event) -> ControlFlow<()> + Send` callback with an `EventSet` subscription                | Mirrors the bridge's tagged dispatcher, and `Break` stops at once                                                          |
| Stop              | `StopHandle` stops every solve attached to it, and a stop is sticky                                  | Behaves like a cancellation token, so sharing one handle never loses a stop                                                |
| Solution payload  | `CpSolverResponseView`, zero-copy                                                                    | Callers that keep a solution call `to_owned_message`                                                                       |
| Generated lints   | `#[expect(clippy::all, clippy::pedantic, ...)]` on `mod proto`                                       | Buffa follows its own lint policy                                                                                          |

## Milestones

| Phase | Deliverable                                                       | Status                                                   |
| ----- | ----------------------------------------------------------------- | -------------------------------------------------------- |
| M0    | Scaffolding, lints, formatters, `justfile`                        | Done                                                     |
| M1    | Buffa types and round-trip tests in `tests/protobuf.rs`           | Done                                                     |
| M2    | Bridge over `SolveWrapper` and `CpSatHelper`                      | Done                                                     |
| M3a   | Prebuilt and `dir:` modes, shared cache, runtime copy             | Done on Windows, Linux in CI                             |
| M4    | `solve`, `Solver`, `StopHandle`, helpers, diagnostics             | Done                                                     |
| M5    | Parity matrix in `docs/parity.md`, cross-checked with Python 9.15 | Done                                                     |
| M6    | CI on Windows, Ubuntu 24.04, and Ubuntu 26.04                     | Done                                                     |
| M3b   | Source backend                                                    | Implemented, configure step verified, full build pending |
| M7    | Benchmarks                                                        | Open                                                     |
| M8    | Static linking and CP-SAT-only exploration                        | Optional                                                 |

### M3b source backend

`build/source.rs` holds the CMake options that turn off what CP-SAT does not
need. Configuring with them succeeds on Windows with SCIP, HiGHS, and Coin-OR
off. MathOpt must stay on, and Gurobi and Xpress stay on because OR-Tools loads
them at runtime without build dependencies. Inspect the configured targets
instead of trusting that these options produce a CP-SAT-only build. The source
backend must pass the same tests as the prebuilt one, through the on-demand
`source` workflow.

### M7 performance

Benchmark Buffa encode and decode, model transfer and parsing, solve overhead,
response serialization, callback latency and throughput, and view decoding.
Compare with direct C++ and Python on the same models. Add zero-copy paths only
when a benchmark shows a real bottleneck, and never at the cost of safety or
parity.

### M8 optional native work

Static linking of a reduced source build, a CP-SAT-only target, smaller
distribution packages, and borrowed native solution buffers. Measure the benefit
before maintaining any of them.

## Definition of done

- `cargo build` works on clean Windows and Ubuntu 24.04 and 26.04 machines with
  a C++ compiler, `curl`, and `tar`.
- Native libraries are acquired, linked, and found at runtime automatically.
- Buffa types are the only model representation, and models stay mutable between
  solves.
- Every targeted Python capability passes its parity test.
- Callbacks and cancellation are safe.
- `just ci` passes, and normal builds regenerate neither Protobuf code nor
  OR-Tools.
