# Development

## Prerequisites

Install the tools required by the workspaces you modify:

- Rust and Cargo.
- The nightly Rust toolchain with `rustfmt` for the repository's import and impl-item formatting rules.
- [cargo-nextest](https://nexte.st) and [cargo-insta](https://insta.rs).
- Node.js and pnpm for TypeScript packages.
- The .NET SDK for C# projects and proxy tests.
- `wasm-bindgen-cli` for WebAssembly tests.
- A local checkout of [op-lint](https://github.com/ayecue/op-lint) for repository structure and naming checks.

```sh
rustup toolchain install nightly --component rustfmt
rustup target add wasm32-unknown-unknown
cargo install --locked cargo-nextest cargo-insta
cargo install --locked wasm-bindgen-cli --version 0.2.129
make install
```

`make install` runs `pnpm install` in the TypeScript workspaces and restores .NET tools. The op-lint tasks read `OP_LINT_ROOT`, which defaults to `../op-lint`.

## Repository checks

Run commands from the repository root. The `Makefile` invokes [reman](../tools/reman/README.md), which selects the relevant workspaces and packages.

| Command | Purpose |
|---|---|
| `make lint` | Run op-lint, rustfmt, Clippy, ESLint, Prettier, and CSharpier checks. |
| `make format` | Apply the configured Rust, TypeScript, and C# formatters. |
| `make test-unit` | Run Rust source and doc tests, TypeScript tests, and .NET tests. |
| `make test-integration` | Run configured Rust integration, WebAssembly, proxy, and VS Code extension tests. |
| `make test` | Run unit and integration suites. |
| `make test-game` | Run tests against an installed Grey Hack game. Requires `GREY_HACK_PATH`. |

Use `make help` to list language-specific targets. `REMAN_FLAGS` passes additional options to reman:

```sh
make test REMAN_FLAGS=--since
make lint REMAN_FLAGS=--dry-run
```

`--since` selects packages changed since the last release and their dependents.

## Individual workspaces

Rust workspaces build from their own directories:

```sh
cd greybel/rs
cargo nextest run --workspace
cargo test --workspace --doc
cargo clippy --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

The other Rust workspaces are `apps/rs`, `duplex/rs`, `text-mesh/rs`, `assets/rs`, `mono/rs`, and `common/rs`.

TypeScript workspaces use pnpm. Install shared dependencies first and applications last:

```sh
cd common/ts && pnpm install
cd ../../duplex/ts && pnpm install && pnpm test
cd ../../greybel/ts && pnpm install && pnpm test
cd ../../apps/ts && pnpm install && pnpm test
```

See the workspace READMEs for package-specific build and regeneration commands.

## Formatting

- Rust uses nightly rustfmt, followed by op-lint's padding rules.
- TypeScript uses ESLint fixes and Prettier, including the repository's early-exit and padding rules.
- C# uses CSharpier; files excluded by `.csharpierignore` are not formatted.
- Generated `*.generated.ts` and `*.g.cs` files remain in generator output format.

Use `make format-rust`, `make format-ts`, or `make format-cs` to format one language. Use the corresponding lint target to check without editing.

## Build output

All Cargo workspaces share the repository's `target/` directory through `.cargo/config.toml`. TypeScript WebAssembly builds write to `target/wasm/`.

Binary and example names must be unique across Cargo workspaces because they share the same output directories.

## Change files

Every release-relevant change to a reman project requires a change file. This repository excludes tests, benchmarks, examples, and README files from release accounting:

```sh
cargo reman change rs/greybel-core:minor -m "Describe the change"
```

Use `major`, `minor`, or `patch` for released changes. Use `none` when no release is required:

```sh
cargo reman change rs/greybel-core:none -m "Update internal tests"
```

Run `cargo reman check` to find released projects with changes that no pending change file describes. See the [reman reference](../tools/reman/README.md) for change-file syntax and project identifiers.

## Releases

Prepare a release locally, review the commit, and push it to `main`:

```sh
cargo reman check
cargo reman plan
cargo reman release prepare
git commit -am "release"   # versions, changelogs, .changes/release.toml
git push
```

`plan` previews version changes. `release prepare` updates versions, changelogs, and the release record. Pushing a commit that changes `.changes/release.toml` starts the release workflow ([CI and releases](ci.md)); it runs `cargo reman release publish --draft`, builds and attaches each app's files, publishes greybel-vs to the marketplaces, and publishes the drafts once everything succeeded.

Use `--dry-run` before preparation or publishing to inspect the operations without applying them.

## Paths and generated files

Do not use relative `../` chains that leave a workspace to locate repository resources:

- Rust uses environment values configured in `.cargo/config.toml`, such as `PROTOCOLS` and `DUPLEX`.
- .NET uses properties from `Directory.Build.props`, including `GreybelRust`.
- TypeScript tests use `inject(name)` with paths configured by the workspace.

Generated protocols, TypeScript modules, and C# files are refreshed by their owning tests or build commands. Do not edit generated output without changing its source definition or generator.
