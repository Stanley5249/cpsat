# Conventions

Rules for doc comments, examples, and tests. They adapt the C++ and Python
CP-SAT APIs at the OR-Tools version that `docs/parity.md` pins.

## Sources

- `cp_model.proto` comments define what each constraint means.
- C++ `cp_model.h` gives a one-line summary, often a formula, such as
  `target == min(exprs)`.
- Python `cp_model.py` adds parameter details under `Args:` and error conditions
  under `Raises:`.
- Neither has per-method examples, so examples come from the official samples in
  `ortools/sat/samples`.

## Public items

```rust
/// `exprs[index] == target`.
///
/// CP-SAT accepts affine expressions `a * x + b` for `index` and `target`.
///
/// # Panics
///
/// If ...
///
/// # Examples
///
/// ...
#[doc(alias = "add_element", alias = "AddElement")]
```

- **Summary.** One sentence. State the meaning as a formula where one exists,
  following C++, and fold Python's `Returns:` into it.
- **Body.** CP-SAT restrictions, such as affine-only arguments, the Protobuf
  encoding where the caller sees it, and behavior that differs from Python. Name
  parameters in backticks in the prose. The signature documents their types, so
  there is no `Args` section.
- **Errors and panics.** `# Errors` and `# Panics` appear only when the item can
  fail, and follow the Failure rule in `docs/design.md`. Each Python `Raises:`
  condition maps to one of them or to `validate`.
- **Aliases.** `#[doc(alias)]` lists the Python and C++ names that differ from
  the Rust name, so rustdoc search finds the item under either. Aliases change
  only the documentation. The prose mentions Python only where the behavior
  differs, and a change to a mirrored item updates its `docs/parity.md` entry in
  the same commit.
- **Presence.** The `missing_docs` lint requires a doc comment on every public
  item. `cpsat-sys` and the generated `proto` module expect the lint, because
  `bridge/bridge.h` and the `.proto` files document their items.

## Examples

- **Where.** The crate root, the entry types `CpModel`, `CpSolver`,
  `Constraint`, `LinearExpr`, and `Domain`, and items whose argument shape is
  not obvious from the signature, such as table rows, automaton transitions,
  circuit arcs, intervals, and reservoir events.
- **What runs.** Doctests build a model and do not solve it, because solve
  results belong to the tests. The crate root and `README.md` show a `no_run`
  solve.
- **Content.** Each example adapts the matching official sample and names it in
  a comment. Without a sample, use the example in the `cp_model.proto` comment,
  and otherwise the smallest construction.
- **Repeated variables.** Examples and tests create a fixed number of variables
  with `std::array::from_fn(|_| model.new_var(...))`, destructured as
  `let [x, y, z]` where possible, and a number that depends on data with
  `(0..n).map(...).collect()`.
- **`crates/cpsat/examples/`** holds only `solve.rs`, which CI runs as the
  deployment check. Ported samples live in `tests/e2e/samples.rs`.

## Internal items

- Each module starts with a `//!` line on its purpose, plus its dependency limit
  when it has one.
- A private item has a one-line `///` that states its result or invariant,
  without sections.
- `//` comments say why the code is the way it is.

## Tests

The tests check the binding. OR-Tools tests the solver, so search strategies,
optimal values beyond what a sample prints, and statistics beyond range checks
are out of scope.

- **`model.rs` and `domain.rs`** cover pure Rust logic, such as `LinearExpr`
  merging and overflow and `Domain` operations.
- **`equivalence.rs`** checks that each constraint family writes the same
  `CpModelProto` as Python, from fixtures that `just gen-fixtures` regenerates.
  A new family gets a fixture.
- **`constraints.rs`** solves each family once and checks feasibility and one
  property of its meaning.
- **`samples.rs`** ports official samples and asserts what the Python sample
  prints, checked by running it with the pinned `ortools`. Each constraint
  family and solver feature with an official sample has one ported, preferring
  samples that exist in both C++ and Python. Families without one rely on
  `equivalence.rs` and `constraints.rs`.
- **The other files** cover the binding's own mechanics: encoding, the FFI,
  events, callback panics, stops across threads, and response reading. These get
  full coverage, because nothing upstream tests them.
