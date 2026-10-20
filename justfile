set minimum-version := "1.53.0"
set default-list
set unstable
set lists

[windows]
set shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# Format all sources
[parallel]
fmt: _fmt-cargo _fmt-tombi _fmt-oxfmt _fmt-just

# Format, then apply Clippy fixes
fix: fmt
    cargo clippy --fix --allow-dirty --workspace --all-targets

# Recipes list features instead of --all-features, which would turn on
# build-libortools and build OR-Tools from source.

# Run Clippy with warnings as errors
lint *args:
    cargo clippy --quiet --locked --workspace --all-targets --features cpsat/json {{ args }} -- -D warnings

# Fast local gate: format checks and lints
[parallel]
check: (_fmt-cargo "--check") (_fmt-tombi "--check") (_fmt-oxfmt "--check") (_fmt-just "--check") lint

# Run tests
test *args:
    cargo test --locked --workspace --features cpsat/json {{ args }}

# Build rustdoc with warnings as errors
[env("RUSTDOCFLAGS", "-D warnings")]
doc *args:
    cargo doc --locked --workspace --features cpsat/json --no-deps {{ args }}

# Full gate
ci: check test doc

# Regenerate Buffa code from the vendored protos
[group("proto")]
[working-directory("crates/cpsat/proto")]
gen-proto *args:
    buf generate {{ args }}
    cargo fmt -p cpsat

# Fail when the committed Buffa code differs from a fresh generation
[group("proto")]
check-proto: gen-proto
    git diff --exit-code -- crates/cpsat/src/proto

# Regenerate the Python models that tests/e2e/equivalence.rs compares against
[group("fixtures")]
gen-fixtures:
    uv run --quiet --with ortools==9.15.6755 python -I crates/cpsat/tests/e2e/fixtures/generate.py

# Fail when the committed fixtures differ from a fresh generation
[group("fixtures")]
check-fixtures: gen-fixtures
    git diff --exit-code -- crates/cpsat/tests/e2e/fixtures

_fmt-cargo *args:
    cargo fmt --all {{ args }}

_fmt-tombi *args:
    tombi format --quiet {{ args }} .

_fmt-oxfmt *args:
    bunx oxfmt {{ args }}

_fmt-just *args:
    just --fmt {{ args }}
