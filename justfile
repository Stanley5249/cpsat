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

# Type-check every target
typecheck *args:
    cargo check --locked --workspace --all-targets {{ args }}

# Run Clippy with warnings as errors
lint *args:
    cargo clippy --locked --workspace --all-targets {{ args }} -- -D warnings

# Fast local gate: format checks and lints
check: (_fmt-cargo "--check") (_fmt-tombi "--check") (_fmt-oxfmt "--check") (_fmt-just "--check") lint

# Run tests
test *args:
    cargo test --locked --workspace {{ args }}

# Build rustdoc with warnings as errors
[env("RUSTDOCFLAGS", "-D warnings")]
doc *args:
    cargo doc --locked --workspace --no-deps {{ args }}

# Full gate
ci: check test doc

# Regenerate Buffa code from the vendored protos
[working-directory("crates/cpsat/proto")]
gen:
    buf generate
    cargo fmt -p cpsat

_fmt-cargo *args:
    cargo fmt --all {{ args }}

_fmt-tombi *args:
    tombi format --quiet {{ args }} .

_fmt-oxfmt *args:
    bunx oxfmt {{ args }}

_fmt-just *args:
    just --fmt {{ args }}
