# TypeScript apps

This pnpm workspace contains the released TypeScript applications. Its `pnpm-workspace.yaml` links packages from [`common/ts`](../../common/ts), [`duplex/ts`](../../duplex/ts), and [`greybel/ts`](../../greybel/ts); install and build those workspaces first.

| App | Purpose |
|---|---|
| [`greybel-vs`](greybel-vs) | VS Code extension, packaged for desktop platforms with binaries from [`apps/rs`](../rs) and for the web with WebAssembly. |
| [`greybel-editor-web`](greybel-editor-web) | CodeMirror web editor with language features, shared-world execution and debugging, transformation, API documentation, and project sharing. |
| [`greybel-api-reference-site`](greybel-api-reference-site) | Standalone API reference and scripting guide. Use `?filter=` to filter the reference and `?tab=guide` to open the guide. |

## Commands

Run from `apps/ts`:

```sh
pnpm install
pnpm test        # Vitest; greybel-vs also has test:desktop and test:web
pnpm typecheck
pnpm build       # build every app into its dist/
pnpm format
pnpm format:check
```

The VS Code extension's end-to-end suites run separately:

```sh
pnpm --filter greybel-vs test:desktop
pnpm --filter greybel-vs test:web
```

Set `EDITOR_SERVICE_URL` when building `greybel-editor-web` to enable saving shared projects to an editor service. Without it, the editor still supports projects encoded directly in share links.
