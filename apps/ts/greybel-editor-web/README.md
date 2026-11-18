# greybel-editor-web

A browser-based GreyScript editor with language-server features, multi-file projects, execution, debugging, transformation, API documentation, and shareable projects. Language processing, builds, runtimes, and the simulated world run in WebAssembly workers.

## Features

- **Files and tabs:** create, rename, and delete nested files; select an entry file; retain each file's editing state and breakpoints when its tab closes.
- **Settings:** the project's `greybel.json` holds the settings of runs, transforms and diagnostics, as in the CLI and greybel-vs. It is listed with its template until its first edit creates it, and edited with completion and hovers from its schema; a problem greybel-lsp reports in it is shown on its line, and the last valid settings stay in use until it is fixed.
- **Editing:** CodeMirror 6 with semantic highlighting, completion, hover, signature help, diagnostics, definitions, references, rename, and formatting across project files. The signatures and code examples in hovers, completions and signature help are highlighted.
- **Run and debug:** run with the `run` settings of `greybel.json`, set breakpoints in any file, inspect stacks and variables, and evaluate code while stopped.
- **Terminals:** run programs in multiple terminals, each its own script (the entry file by default) with its own `greybel.json` profile. Their runtimes share one persistent simulated world generated from `run.seed`; **New world** stops current programs and regenerates it. **Copy** copies a terminal's text without its colors and other rich-text formatting.
- **Transform and reference:** transform the current file with the `build` settings of `greybel.json`, and browse or insert API examples.
- **Loading:** the toolbar shows the parts still loading their WebAssembly, and those that failed to; Run, Debug and Transform wait until their parts are loaded.
- **Layout:** splitters resize the file list, the code, the API reference, the panels and the debugger, dragged or with the arrow keys; the sizes are stored in browser local storage, and a double click resets one.
- **Sharing and storage:** share a project in a `?c=` link or save it to an optional editor service for an `?id=` link. The last edited project is stored in browser local storage.

Imports use project paths such as `import_code("/lib/util.src")` or paths relative to the importing file such as `#include "util.src"`.

Single-file `main.src` projects use greybel-js's plain-code storage format. Multi-file projects use a versioned JSON project document; both formats work in shared links and saved projects.

## Development

Run these commands from `apps/ts` after installing and building the linked TypeScript workspaces:

```sh
pnpm --filter "greybel-editor-web^..." build   # build dependent workers and WebAssembly
pnpm --filter greybel-editor-web dev           # start Vite
pnpm --filter greybel-editor-web build         # write the static site to dist/
pnpm --filter greybel-editor-web preview       # serve the production build
pnpm --filter greybel-editor-web test          # build in test mode and run Vitest/Playwright in Chromium
```

`EDITOR_SERVICE_URL` enables **Save**, which stores the project in [greybel-editor-service](../../rs/greybel-editor-service/README.md) and opens it again from `?id=`. `.env.production` sets it to the deployed service for `build`; `.env.test` sets an address the tests answer themselves. Override it at build time:

```sh
EDITOR_SERVICE_URL=http://localhost:8080 pnpm --filter greybel-editor-web build
```

`WASM_BASE_URL` points the production build's `.wasm` URLs at another folder; CI sets it to the folder on DigitalOcean Spaces it uploads them to ([CI and releases](../../../docs/ci.md)).
