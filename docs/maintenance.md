# Maintenance

Checklists for the recurring upgrades. A step that names a CI check fails the
build when it is missed.

## Bump OR-Tools

1. Update every URL and SHA-256 in `crates/cpsat-sys/build/artifacts.rs`,
   following its module comment.
2. Copy `cp_model.proto` and `sat_parameters.proto` from the new tag into
   `crates/cpsat/proto/ortools/sat`, then run `just gen-proto`. CI compares the
   files with the tag.
3. Set the `ortools==` pin in the `gen-fixtures` recipe to the full version in
   the archive names, such as `9.15.6755`, then run `just gen-fixtures`. CI
   compares the pin with the tag.
4. Compare `OPTIONS` in `crates/cpsat-sys/build/source.rs` with the new root
   `CMakeLists.txt` defaults.
5. Recheck the facts section of `docs/design.md` and the reference links in
   `docs/parity.md`, and update the version in the build metadata example in
   `docs/roadmap.md`.
6. Search for the old version, such as `rg 9\.15`, to find test comments that
   cite Python's output, and rerun the samples they name.
7. Run `just ci`, then the `source` workflow.

## Bump Buffa

1. Set the same version for `buffa` in the root `Cargo.toml` and for the plugin
   in `crates/cpsat/proto/buf.gen.yaml`. CI compares the two.
2. Run `just gen-proto`, then `just ci`.
3. Read the Buffa changelog for changes to proto2 defaults, and update
   `docs/buffa-proto2-defaults.md`.

## Add a target

1. Add the official archive to `crates/cpsat-sys/build/artifacts.rs` and its
   `prebuilt` match.
2. Extend `toolchain`, the library filter in `place_runtime`, and `link` in
   `crates/cpsat-sys/build.rs`.
3. Add the target's rpath to `.cargo/config.toml`, such as `@loader_path` on
   macOS, and to the deployment section of `docs/build.md`.
4. Add the target to the `test` matrix in `.github/workflows/ci.yml` and to the
   supported targets in the README.
