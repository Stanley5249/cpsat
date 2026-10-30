# Contributing

The README lists the tools to install, and `just` lists the recipes.

## Changes

- Open an issue before adding public API or changing a decision in `docs/design.md`, so the design is agreed before the code.
- Follow the rules in `docs/design.md` for public API, and `docs/conventions.md` for doc comments, examples, and tests.
- Change the generated code in `crates/cpsat/src/proto` through `just gen-proto`, and edit the `.proto` files only when bumping OR-Tools.

## Before a commit

Run `just check` before a commit, and `just ci` before a push or a pull request. After changing the protos or the fixtures, also run `just check-proto` or `just check-fixtures`.

## Commits

Commits follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/):

```text
<type>[(<scope>)]: <description>

[body]
```

- Write the description as an imperative phrase. Aim for a subject under 50 characters, and keep it within 72.
- Add a body only when the subject leaves the reason unclear. Keep it to two short paragraphs or five bullets, with lines of 72 characters or fewer.
- Make one commit per action: a feature with its tests and docs, or a single fix, move, rename, refactor, reformat, or dependency update.

### Types

| Type       | Use for                                                      |
| ---------- | ------------------------------------------------------------ |
| `feat`     | New public API or behavior                                   |
| `fix`      | A bug fix                                                    |
| `perf`     | A faster or leaner path with the same behavior               |
| `refactor` | A code change with the same behavior, including renames      |
| `test`     | Tests and fixtures only                                      |
| `docs`     | Documents and doc comments only                              |
| `style`    | Formatting and rewrapping only, in code, docs, or manifests  |
| `build`    | Manifests, build configs, the build script, and the justfile |
| `ci`       | `.github/workflows`                                          |
| `chore`    | Lint, formatter, and editor configs, and anything else       |
| `revert`   | A revert of an earlier commit                                |

In the build script, `fix`, `perf`, and `refactor` take precedence over `build`. Build configs are `rust-toolchain.toml` and `.cargo/`. Lint and formatter configs are `clippy.toml`, `tombi.toml`, and `.oxfmtrc.json`.

### Scopes

| Scope       | Covers                                                        |
| ----------- | ------------------------------------------------------------- |
| `cpsat`     | `crates/cpsat`                                                |
| `cpsat-sys` | `crates/cpsat-sys`, including the C++ bridge and build script |
| none        | Both crates at once, or only repository files                 |

The scope follows the crate a change touches, whatever its type. Repository files include `docs/`, the README, the justfile, and the workspace `Cargo.toml`. Write `ci` commits without a scope, as `ci: <description>`.

### Breaking changes

Until the first crates.io release, change interfaces in place and leave out `!` and the `BREAKING CHANGE:` footer. After it, mark each breaking change with both.

### Examples

```text
feat(cpsat): add the circuit constraint
build(cpsat-sys): choose OR-Tools by features
docs: link the docs with absolute URLs
```

## Branches

Name a branch `<type>/<topic>` with a short kebab-case topic, such as `feat/reservoir` or `fix/domain-union`.

## License

Unless you state otherwise, any contribution you submit for inclusion, as defined in the Apache-2.0 license, is dual licensed as in the README, without additional terms or conditions.
