# greybel-editor-web

A browser-based GreyScript editor with language-server features, multi-file projects, execution, debugging, transformation, API documentation, and shareable projects. Language processing, builds, runtimes, and the simulated world run in WebAssembly workers.

## Features

- **Files and tabs:** create, rename, and delete nested files; select an entry file; retain each file's editing state and breakpoints when its tab closes.
- **Editing:** CodeMirror 6 with semantic highlighting, completion, hover, signature help, diagnostics, definitions, references, rename, and formatting across project files.
- **Run and debug:** run from the entry file with program parameters, set breakpoints in any file, inspect stacks and variables, and evaluate code while stopped.
- **Terminals:** run programs in multiple terminals. Their runtimes share one persistent simulated world; **New world** stops current programs and regenerates it from the seed.
- **Transform and reference:** transform the current file as written, uglified, or beautified, and browse or insert API examples.
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

Set `EDITOR_SERVICE_URL` at build time to enable saving projects to an editor service:

```sh
EDITOR_SERVICE_URL=https://example.com pnpm --filter greybel-editor-web build
```

The service must accept `POST /code` with `{ "content": "..." }` and return `{ "id": "..." }`, and serve `GET /code/<id>` as `{ "content": "..." }`.
