# Rust CP-SAT binding plan

Rust owns the model. Protobuf defines the contract. C++ executes the solver. The
FFI exposes only capabilities that Rust cannot implement directly.

## Objectives

Build a Rust-native binding for Google OR-Tools CP-SAT with at least the
functional capabilities of the official Python CP-SAT binding, through an
idiomatic Rust modeling API that writes into the Protobuf model.

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

- A literal copy of Python's API, such as the Pandas series constructors.
  Iterators and collections cover the same needs.
- A persistent incremental solver.
- OR-Tools subsystems outside CP-SAT.
- Custom propagation or search algorithms.
- A CP-SAT-only source fork.

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
crates/cpsat          Model, Buffa types, Solver, StopHandle, diagnostics
   |  (safe Rust, owns the model)
crates/cpsat-sys      build.rs, raw extern "C" declarations, links = "ortools"
   |  (serialized bytes, fn pointer + context, opaque session)
cpsat-sys/native      extern "C" over SolveWrapper and CpSatHelper
   |
ortools.dll / libortools.so  (SolveCpModel)
```

Rust owns the model and parameter representation, serialization, every model
change, callback handling, and error interpretation. C++ owns solver execution,
callback registration, the active solve and its stop mechanism, and native
diagnostics. A native session lives for one solve and never holds a second copy
of the model.

## Rules

These rules apply to every change. The Decisions tables below record one-time
choices.

| Rule           | Statement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reason                                                                                                                                                                                  |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Protobuf types | Wrap a message only where its encoding is weakly typed: indices, `-i - 1` literals, flat interval lists, and `oneof` constraints become `Model`, handles, `LinearExpr`, `Constraint`, and `Domain`. Use typed records as generated, such as `SatParameters`, `CpSolverResponse` and its view, status and strategy enums, and the variable and constraint messages that indexing returns, and add methods to them                                                                                                                                                                                                                                                                                                                                                                                                                  | A wrapper adds type safety where the schema has none, and elsewhere it would duplicate hundreds of fields or copy them on every call                                                    |
| Paths          | Every public item has one path. `prelude` holds every hand-written item except `Error` and `Result`, which stay at the crate root. `proto` holds the generated types, and `buffa` re-exports their runtime, so `Message` is `cpsat::buffa::Message`. Every other module is private                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | One import path per item, a glob that never shadows `std`'s `Result`, and a prelude whose members change only with the hand-written API, not with the schema                            |
| Entry points   | One method per meaning. A constructor whose return type differs may name its type, such as `int_var` and `bool_var`, or `constant` and `bool_constant`. Methods with one meaning over several input types are one method over a sealed trait, with an associated type wherever the input fixes the type of a value or result, such as `value` over `Evaluate`, `minimize` and `maximize` over `Objective`, and `add_hint` over `Hint`. A method takes `impl IntoIterator` instead of a single-item twin, and conversions use `From`/`Into`. A wrapper may add `into_proto` beside its `From`, as `String::into_bytes` sits beside `From<String>`. The one exception is `Model` forwarding `solve`, `validate`, and `stats` to its message, which the Methods decision keeps so both a `Model` and a bare `CpModelProto` can solve | Fewer names to learn, the compiler rejects a wrong pairing such as a `bool` hint on an integer variable, and the input type replaces Python's runtime dispatch                          |
| Allocation     | Reads, predicates, and lookups allocate nothing, such as `value` on a handle, `Domain::is_included_in`, and `Model::to_literal`. An operation that returns a new value allocates once for it and builds no intermediate collections. The exceptions are building owned model data (expressions, constraints, and the `Vec` fields a constructor collects from an iterator), clones the schema needs because it stores one expression twice, encoding the model and parameters for each solve, and rarely called accessors that return an owned `LinearExpr` from a borrowed message, such as `Model::interval_start`                                                                                                                                                                                                              | Solution callbacks and model-building loops call these paths often, and allocation is the main cost a thin wrapper can add                                                              |
| Ownership      | Data the model stores is taken by value through `impl Into<T>`, and read-only data is borrowed, such as `Solver::new(&SatParameters)` and `Model::solve(&self)`. A method takes ownership only when it keeps the value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | The caller decides when to clone, and the API adds no hidden copies                                                                                                                     |
| Failure        | A caller's programming error panics, with a `# Panics` section on the public item: `i64` overflow, a table row of the wrong length, or a handle of the wrong kind. A broken internal invariant, such as the bridge returning bytes that are not a response, panics with a message naming the invariant. A native failure the caller can cause returns `Error`. `validate` returns `Error::InvalidModel` for an invalid model, while `solve` reports one through the `MODEL_INVALID` status. The modeling API checks eagerly only what the validator cannot see, such as a ragged table row that the flat schema would silently split into other rows                                                                                                                                                                              | One check per mistake, in the place that can see it, and no silent wrong model                                                                                                          |
| Lookup         | A raw schema index returns `Option`, such as `int_var_from_index`. Indexing with a typed handle works like a slice and panics when the model lacks it, while a method that asks about a handle returns `Option`, such as `value` and `to_literal`. Accessors follow the Allocation rule                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | An index comes from data and may be absent, while a handle comes from the model and a missing one is a bug                                                                              |
| Builders       | A value configured before it enters the model takes `self` and returns `Self` with `#[must_use]`, such as `Constraint::only_if`, `with_name`, and `Solver::on_event`. A change after insertion goes through `model[handle]`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Configuration reads as one expression, and a dropped builder is a warning                                                                                                               |
| `#[must_use]`  | Every public method without side effects and every builder method carries it. A model method that appends, such as `int_var`, does not, because a caller may want only the new variable in the message                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Warns on a discarded result without flagging deliberate appends                                                                                                                         |
| Python mapping | A public item that mirrors Python says so in its documentation, as "like Python's `name`". A change to such an item updates its `docs/parity.md` row and test citation in the same commit                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Migration from Python stays a lookup, and the parity table stays true                                                                                                                   |
| Encoding tests | A new constraint family or encoding gets a Python fixture in `tests/e2e/equivalence.rs` for its message and a solve test for its meaning                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | The fixture proves the bytes match Python, and the solve proves they mean what the name says                                                                                            |
| Visibility     | `pub` or private, except `pub(crate)` on handle indices, expression terms and evaluation, `constraint::linear`, and `EventSet::bits`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Handles, expressions, constraints, and responses share these internals across sibling modules, and a common parent module would merge most of the crate without changing the public API |

## Decisions

| Topic             | Decision                                                                                                                                                                              | Reason                                                                                                                                                                                              |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Crates            | `cpsat-sys` and `cpsat` under `crates/`                                                                                                                                               | Cargo's `links` key allows one native link per graph                                                                                                                                                |
| Codegen           | `just proto-gen` runs `buf generate` with the remote Buffa plugin, and the output is committed                                                                                        | Buffa's recommended path. Normal builds need no `buf` or `protoc`                                                                                                                                   |
| Bridge build      | `cc` crate with definitions read from `ortoolsTargets.cmake`                                                                                                                          | Prebuilt mode needs only a C++ compiler, not CMake                                                                                                                                                  |
| Download          | `curl` and `tar` from the system                                                                                                                                                      | No build-dependency compile time, and the system proxy and CA store apply. `build/fetch.rs` names when to switch to `ureq`                                                                          |
| Cache             | Archives, extractions, and source builds in `CPSAT_CACHE_DIR`, each filled once under a `File::lock`                                                                                  | Survives `cargo clean`, one extraction serves every `OUT_DIR`, and concurrent builds wait instead of racing                                                                                         |
| MSRV              | 1.89                                                                                                                                                                                  | `File::lock`                                                                                                                                                                                        |
| Runtime libraries | Hard-linked, or copied across volumes, into `OUT_DIR/runtime`                                                                                                                         | See the `cpsat-sys` build script                                                                                                                                                                    |
| Errors            | `thiserror` enum in `cpsat`, `anyhow` in the build script                                                                                                                             | Callers can match an invalid model or a native failure, and the build script reports a context chain                                                                                                |
| Solve API         | `Model::solve` or `CpModelProto::solve` for the plain case, and a `Solver` that borrows its parameters for events and stops                                                           | Mirrors Python's `CpSolver` without a mandatory object for the plain case                                                                                                                           |
| Methods           | Operations on a model or a response are methods on `Model`, `CpModelProto`, and `CpSolverResponse`, and `Model` forwards to its message. Only `default_parameters` is a free function | Like Python, and code that borrows a message needs no `Model` or clone                                                                                                                              |
| Events            | One `FnMut(Event) -> ControlFlow<()> + Send` callback with an `EventSet` subscription, stored as the generic parameter of `Solver`                                                    | Mirrors the bridge's tagged dispatcher, and `Break` stops at once. A boxed callback would give `Solver` a nameable type, but its drop glue keeps the closure's borrows alive until the solver drops |
| Stop              | `StopHandle` stops every solve attached to it, and a stop is sticky                                                                                                                   | Behaves like a cancellation token, so sharing one handle never loses a stop                                                                                                                         |
| Model transfer    | Each solve encodes the model once in Rust and parses it once in C++                                                                                                                   | Rust owns the model. Skipping the copy needs a native model that lives across solves, so M7 measures the cost first                                                                                 |
| Solution payload  | `CpSolverResponseView`, zero-copy                                                                                                                                                     | Callers that keep a solution call `to_owned_message`                                                                                                                                                |
| Formats           | Buffa `text=true` and `json=true` with `gate_impls=true`, as the `text` and `json` features                                                                                           | Pure Rust. `views` is a required feature, because Buffa gates views with the formats                                                                                                                |
| Defaults          | `default_parameters` fills `SatParameters` through C++ reflection                                                                                                                     | Buffa generates no proto2 default getters                                                                                                                                                           |

## Modeling API

`Model` owns a `CpModelProto` and is its only persistent representation. Each
model operation appends to that message, and `Model::proto_mut` reaches every
field the API does not cover.

```text
model.int_var(0..=10) -----------------------> IntVar(i32)
model.bool_var() ----------------------------> BoolVar(i32), !b = -i - 1
x + 2 * y - 3 -------------------------------> LinearExpr(LinearExpressionProto)
(x + 2 * y).le(10).only_if(b) ---------------> Constraint(ConstraintProto)
Constraint::circuit([(0, 1, a)]) ------------> Constraint
model.add(constraint) --- move into model ---> ConstraintRef(i32)
```

| Topic          | Decision                                                                                                                                                                                                                                                                                                                                                   | Reason                                                                                                                                                                                                                               |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Names          | Python's name wherever a Rust convention allows it, such as `constant`, `value`, and `parameters`, without `new_` on constructors such as `int_var`, and without `add_` on constraint constructors, while `Model` methods such as `add_hint` keep it. Variable constructors name their kind, as `int_var` does, and elsewhere integer is the unmarked kind | Migration from Python stays a lookup, and Python and C++ leave integer unmarked                                                                                                                                                      |
| Handles        | `IntVar`, `BoolVar`, `IntervalVar`, and `ConstraintRef` are typed `i32` indices without a model identity                                                                                                                                                                                                                                                   | Like `petgraph` and `slotmap`, the types keep categories apart at no cost, and the model only appends                                                                                                                                |
| Literals       | `BoolVar` is a literal, and `!` negates it                                                                                                                                                                                                                                                                                                                 | The schema's `-i - 1` convention, and `!b` counts as `1 - b` in an expression                                                                                                                                                        |
| Expressions    | `LinearExpr` wraps `LinearExpressionProto`, and writing it sorts, merges, and drops zero terms                                                                                                                                                                                                                                                             | Matches Python's output, and the arithmetic panics on `i64` overflow where Python wraps                                                                                                                                              |
| Comparisons    | Inherent `eq`, `ne`, `le`, `lt`, `ge`, `gt`, and `in_domain` on `IntVar`, `BoolVar`, and `LinearExpr`                                                                                                                                                                                                                                                      | Rust's comparison traits must return `bool`. Handles still derive `PartialOrd` and `Ord` by index for sorted collections, because a by-value inherent method takes precedence over the trait method of the same name                 |
| Constraints    | `Constraint` owns a `ConstraintProto`, with `only_if` and `with_name`. Every family is a constructor on it                                                                                                                                                                                                                                                 | One common type, and every family in v9.15 accepts enforcement literals                                                                                                                                                              |
| Insertion      | `Model::add` is the only path for constraints, and intervals come from `Model` methods                                                                                                                                                                                                                                                                     | An interval is a constraint that other constraints reference by index                                                                                                                                                                |
| Validation     | The OR-Tools validator through `Model::validate` and the solve status, plus the eager checks the Failure rule allows                                                                                                                                                                                                                                       | Python's other eager argument checks would duplicate the validator                                                                                                                                                                   |
| Compositions   | Python helpers that compose other calls stay compositions: `add_map_domain` as `eq` and `ne` with `only_if`, `values` as a map over `value`, and `weighted_sum` as `zip` and `sum`. Float expressions have no inspection API beyond the objective message                                                                                                  | Each takes a few lines, `docs/parity.md` gives the encoding, and tests pin it to Python's                                                                                                                                            |
| Responses      | `value` takes anything that implements the sealed `Evaluate` trait, whose output type follows the input: `i64` for a handle, constant, or `LinearExpr`, `bool` for a literal, and `f64` for a `FloatExpr`. The owned response and the solution view share it                                                                                               | Mirrors `CpSolver.value`, `boolean_value`, and `float_value` with one method, because the input type already selects the result. A handle reads its slot without building an expression, because callbacks read values on a hot path |
| Proto identity | `tests/e2e/equivalence.rs` compares each family with the output of `tests/e2e/fixtures/generate.py`                                                                                                                                                                                                                                                        | Proves the encoding equals Python's                                                                                                                                                                                                  |

## Milestones

M0 to M6 and M9 are done: the scaffolding, Buffa types, the bridge, the prebuilt
and `dir:` modes, the solver API, the parity matrix, CI, and the modeling API.

| Phase | Deliverable                                | Status                                               |
| ----- | ------------------------------------------ | ---------------------------------------------------- |
| M3b   | Source backend                             | Done on Windows, Linux `source` workflow not run yet |
| M7    | Benchmarks                                 | Open                                                 |
| M8    | Static linking and CP-SAT-only exploration | Optional                                             |

### M3b source backend

`build/source.rs` holds the CMake options that turn off what CP-SAT does not
need, and says why MathOpt stays on. Inspect the configured targets instead of
trusting that these options produce a CP-SAT-only build. A full Windows build
passes the same tests as the prebuilt archive. The on-demand `source` workflow
runs them on Windows and Ubuntu 24.04, and has not run yet.

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

- `cargo build` works on a clean machine of each platform that the README lists,
  with a C++ compiler, `curl`, and `tar`.
- Native libraries are acquired, linked, and found at runtime automatically.
- Models stay mutable between solves.
- Every Python v9.15 modeling capability has a Rust API or a documented Protobuf
  equivalent, and passes its parity test.
- Typical modeling code needs `use cpsat::prelude::*` and the `cpsat::proto`
  types it names.
- Callbacks and cancellation are safe.
- `just ci` passes, and normal builds regenerate neither Protobuf code nor
  OR-Tools.

## Open questions before publishing

A design review on 2026-10-19 found the direction sound and named these
decisions as expensive to reverse after a crates.io release.

- Buffa, a 0.x crate, is part of the public API, so each Buffa minor release is
  a breaking change here. It stays until this decision, and keeping the wire
  bytes as the stable interchange would keep a later switch possible.
- The supported platforms are two x86_64 targets. The README documents `dir:` as
  the offline mode, and the macOS and aarch64 archives that OR-Tools publishes
  are not supported yet.
- Shipped binaries find the OR-Tools libraries only through
  `DEP_ORTOOLS_RUNTIME` and a manual copy, and Linux gets no `$ORIGIN` rpath.
  Shipping them also requires the license notices of OR-Tools and its bundled
  libraries.
- A versioning policy that ties crate releases to OR-Tools releases.
