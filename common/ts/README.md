# Shared TypeScript workspace

This pnpm workspace provides shared event/error utilities, TypeScript and test
configuration, lint presets, and the [`wasm-host`](wasm-host/) helpers used by
other workspaces.

Other TypeScript workspaces reference these packages through `link:` overrides
in their `pnpm-workspace.yaml`. Install this workspace before its dependents;
for example, install `common/ts` and `duplex/ts` before `greybel/ts`.

```sh
pnpm install
pnpm test
pnpm typecheck
pnpm build
pnpm format        # ESLint fixes, then Prettier
pnpm format:check
```

`wasm-host` builds wasm-bindgen crates for Vite workers and supplies the worker
and host helpers that instantiate and serve their WebAssembly. See the
[`greybel/ts` workspace](../../greybel/ts/README.md) for its build and export
conventions.

The `emitter` and `handle-error` packages are copies of axialcode's packages;
avoid unrelated formatting or cleanup changes to that imported code.
