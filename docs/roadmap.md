# Roadmap

## Open work

- **Source mode CMake options.** The `source` workflow passes on Ubuntu.
  `build/source.rs` holds the options that drop what CP-SAT does not need.
  Inspect the configured targets instead of trusting those options.
- **Benchmarks.** Measure Buffa encode and decode, model transfer, solve
  overhead, callback latency, and view decoding against C++ and Python on the
  same models. Add zero-copy paths only for a measured bottleneck.
- **Optional C++ work.** Static linking of a reduced source build, a CP-SAT-only
  target, and borrowed solution buffers. Measure the benefit before maintaining
  any of them.
- **Use in a real project** before the first release.

## Open questions before publishing

Crate releases follow semver and record the OR-Tools version as build metadata,
such as `0.1.0+ortools-9.15`.

These decisions are expensive to reverse after a crates.io release.

- **Buffa in the public API.** Buffa is 0.x, so each of its minor releases is a
  breaking change here. Keeping the wire bytes as the stable interchange keeps a
  later switch possible.
- **Platforms.** Two x86_64 targets for now. OR-Tools also publishes macOS and
  aarch64 archives.
- **Allocation rule.** The rule covers every read, which made
  `Model::to_literal` and `Domain::union_with` more complex than their simpler
  forms, though neither is on a hot path. The benchmarks decide whether to
  narrow it to hot paths.
