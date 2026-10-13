# Testing

## Test suites

Run the standard suites from the repository root:

```sh
make test-unit
make test-integration
make test
```

Use `make test-game` for tests that require an installed Grey Hack game. See [Headless game testing](game-testing.md) for its setup and shared-session controls.

To test only projects changed since the last release and their dependents:

```sh
make test REMAN_FLAGS=--since
```

## Test organization

All languages follow the same conventions:

- Test names describe the expected behavior, such as `should_widen_a_list_on_push` or `ShouldWidenAListOnPush`.
- Each test covers one behavior and follows Arrange, Act, Assert order.
- Keep shared setup and builders out of test case files.
- Prefer public APIs. Use unit tests only when private access is required.
- Keep snapshot assertions in the test function so snapshot names identify the asserting module.

### Rust integration tests

A crate's integration tests form one binary through `tests/main.rs`:

```text
tests/
├── main.rs
├── parse.rs
├── support/
│   ├── mod.rs
│   └── parse.rs
├── fixtures/
└── snapshots/
```

`main.rs` declares the test modules and `support`. Test files import shared helpers with `use crate::support;`. Put helpers used by one test module in `tests/support/<module>.rs` and helpers shared by several modules in `tests/support/mod.rs` or another support module.

Only tests requiring another compilation target, such as WebAssembly tests, use a separate test binary.

From the crate's Rust workspace, run one integration-test module with:

```sh
cargo nextest run -p <crate> <module>::
```

### Rust unit tests

Unit tests mirror the source layout in a neighboring `tests/` directory:

```text
src/ast/atoms.rs
src/ast/tests/atoms.rs
src/ast/tests/support/atoms.rs
```

The source module declares its tests with `#[cfg(test)]` and a `#[path = "tests/atoms.rs"]` module declaration. This gives the tests access to private items without placing test cases in the source file.

Helpers used only by one module belong in its test support directory. Helpers shared by several modules in the same source directory belong in a shared test-support module declared by the parent module.

Methods required only by tests belong in test support code, not in production source.

### TypeScript and .NET

TypeScript tests use `test/support/` for shared helpers. C# tests use `tests/Support/`.

TypeScript test names follow `it('should widen a list on push')`; C# uses names such as `ShouldWidenAListOnPush`.

### Cross-project helpers

Use a workspace test-support crate only for helpers or fixtures shared by multiple projects:

- `greybel-test-support`
- `duplex-test-support`
- `text-mesh-test-support`

Keep helpers used by one crate in that crate's own test support directory.

## Snapshots

Insta snapshots live in `tests/snapshots/`. From the relevant Rust workspace, review changes with:

```sh
cargo insta test --workspace --review
```

To accept all snapshot changes from a nextest run in that workspace:

```sh
INSTA_UPDATE=always cargo nextest run
```

Compatibility fixtures from the TypeScript implementations remain inputs to the Rust tests. The Rust output is stored as a snapshot where applicable. Do not rewrite these fixtures to match new output without reviewing the behavior change.

## WebAssembly tests

From `greybel/rs`:

```sh
cargo test -p greyscript-intrinsics --target wasm32-unknown-unknown --test web
cargo test -p greyhack-world --target wasm32-unknown-unknown --test web
```

The runner must match the `wasm-bindgen` version in `Cargo.lock`. The development setup installs `wasm-bindgen-cli` 0.2.129.

## Proxy and VS Code tests

Run the message-hook client against the .NET proxy from `greybel/rs`:

```sh
cargo nextest run -p greyhack-message-hook-client --features dotnet
```

Run the VS Code extension tests from `apps/ts/greybel-vs`:

```sh
pnpm test:desktop
pnpm test:web
```

The .NET SDK is required for proxy tests. Rebuild affected WebAssembly workers before running extension tests after changing their protocols or services.

## Unity reference fixtures

Text-mesh tests compare output with recorded TextMesh Pro behavior. From `text-mesh/rs`, refresh reference fixtures with `REFRESH_REFERENCE=1` and `cargo test`, not nextest:

```sh
REFRESH_REFERENCE=1 cargo test <test-options>
```

Nextest runs tests in separate processes and would refresh a fixture more than once. Unity reference projects require the Unity version recorded by the project.

## External resources

Tests locate resources outside their package through workspace configuration rather than relative paths that leave the workspace:

- Rust uses values from `.cargo/config.toml`, including `PROTOCOLS` and `DUPLEX`.
- TypeScript uses `inject(name)` and the workspace's configured test paths.
- .NET uses `Directory.Build.props` and assembly metadata for runtime paths.

## Benchmarks

From `greybel/rs`:

```sh
cargo bench -p greybel-core
cargo bench -p greybel-transpiler
cargo bench -p greybel-interpreter
cargo bench -p greybel-type-analyzer
```

The core, transpiler, and type-analyzer benchmarks use `greybel/benches/corpus/`.
