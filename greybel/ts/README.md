# Greybel TypeScript workspace

These packages provide the TypeScript workers, page hosts, protocol bindings,
and TextMate resources for GreyScript and MiniScript. The VS Code extension,
web editor, and API reference applications live in [`apps/ts`](../../apps/ts/).

The workspace uses packages from [`common/ts`](../../common/ts/) and
[`duplex/ts`](../../duplex/ts/) through the `overrides` in
`pnpm-workspace.yaml`. Install those workspaces first, then install this one.
Run these commands in each workspace:

```sh
pnpm install
```

From `greybel/ts`:

```sh
pnpm test
pnpm typecheck
pnpm build          # builds every package's dist/
pnpm format
pnpm format:check
```

## WebAssembly packages

A module imports a Rust crate from the Greybel Cargo workspace as
`crates/<crate>`. `wasm-host` runs Cargo, the wasm-bindgen CLI version locked by
`Cargo.lock`, and Binaryen's `wasm-opt`. Build artifacts stay under Cargo's
build folder; the emitted `<crate>.wasm` is placed next to the JavaScript
bundle and is never inlined. Packages with an export map expose it as
`<package>/<crate>.wasm`; other packages ship it at `<package>/dist/<crate>.wasm`.

`wasmWorker` creates a worker that instantiates the crate and then enters its
`#[wasm_bindgen(start, private)]` function. The host supplies the WebAssembly
to `serveWasm` from `wasm-host/worker` as a URL, bytes, or module. Messages are
queued until instantiation completes. Tests locate external folders through
`inject(name)` and the paths in `config.ts`.

## Generated TypeScript and grammars

The corresponding Rust crate tests check generated TypeScript files and the
API-name rules in the TextMate grammars. From `greybel/rs`:

```sh
cargo test -p <crate>                 # check generated files
UPDATE_TS=1 cargo test -p <crate>     # rewrite generated files
```

For the grammars, use `greyscript-meta` or `miniscript-meta`; see
[`greyscript-textmate`](greyscript-textmate/README.md) and
[`miniscript-textmate`](miniscript-textmate/README.md).
