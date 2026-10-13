# Architecture

Greybel is organized into independent Rust, TypeScript, and .NET workspaces. Each workspace declares its own build and test commands. Local path dependencies connect workspaces without requiring published packages.

## Dependency boundaries

Dependencies point from applications toward libraries:

- `apps/` uses the other projects; no library depends on `apps/`.
- `greybel/` uses `duplex/`, `text-mesh/`, `assets/`, `mono/`, and `common/`.
- `assets/` uses `text-mesh/`.
- `duplex/` uses `common/`.

These boundaries describe library dependencies. The VS Code extension also packages the Rust language-server, runtime, builder, and world-server application binaries.

Rust uses Cargo workspace dependencies, TypeScript uses pnpm links, and .NET uses `Directory.Build.props`.

## Applications

| Application | Purpose |
|---|---|
| `apps/rs/greybel-cli` | `greybel` command-line build, import, execution, REPL, web UI, and MCP commands. |
| `apps/rs/greybel-lsp-stdio` | `greybel-lsp` language server over standard input and output. |
| `apps/rs/greybel-runtime-stdio` | Runtime and debugger service used by the VS Code extension. |
| `apps/rs/greybel-builder-stdio` | Builder and import service used by the VS Code extension. |
| `apps/rs/greyhack-world-server` | Shared Grey Hack world server over TCP. |
| `apps/ts/greybel-vs` | Desktop and web VS Code extension. |
| `apps/ts/greybel-editor-web` | Browser editor. |
| `apps/ts/greybel-api-reference-site` | API reference and scripting guide. |
| `apps/dotnet/greyhack-setup-wizard` | Graphical and command-line Grey Hack setup tools. |

Each application has its own version and release. Rust application binaries are built with `apps/rs/Cargo.lock`.

## Language libraries

The main Rust libraries live in `greybel/rs`.

| Component | Projects | Responsibility |
|---|---|---|
| Language frontend | `greybel-core`, `greybel-core-macros`, `greybel-symbols` | Lexing, parsing, AST storage, dialect rules, and language symbols. |
| Documents and projects | `greybel-documents`, `greybel-project` | Incremental document state, imports, `greybel.json`, profiles, builds, and runs. |
| Document providers | `greybel-documents-fs-provider`, `greybel-documents-memory-provider`, `greybel-documents-host-provider` | Read files from disk, memory, or an editor through the documents protocol. |
| Transpilation | `greybel-transpiler`, `greybel-bundler` | Default, minified, and formatted output; dependency bundling and installer generation. |
| Execution | `greybel-interpreter`, `greybel-interpreter-macros`, `greybel-runtime` | Bytecode compilation, VM execution, native functions, debugging, and editor runtime services. |
| Intrinsics and metadata | `greybel-ms-intrinsics`, `greyscript-intrinsics`, `miniscript-meta`, `greyscript-meta`, `meta-utils` | Language APIs, emulated Grey Hack behavior, and API metadata. |
| Static analysis | `greybel-type-analyzer` | Types, diagnostics, completion, hover, signatures, definitions, and references. |
| Language services | `greybel-lsp`, `greybel-mcp` | LSP request handling and MCP tools. |
| Build services | `greybel-builder` | Editor builds, imports, transformations, and optional in-game deployment. |
| Browser services | `greybel-lsp-web`, `greybel-runtime-web`, `greybel-builder-web` | WebAssembly workers and their TypeScript clients. |
| Input and output | `greybel-io`, `greybel-io-terminal`, `greybel-io-web` | Program input, output batching, terminal I/O, and browser I/O. |
| API reference | `greybel-api-reference`, `greybel-api-reference-web` | Searchable API sections, Markdown descriptions, links, and highlighted examples. |
| Source maps | `greybel-source-map` | Map generated lines to source files and lines. |

TypeScript packages live in [`greybel/ts`](../greybel/ts/README.md). They include browser clients, terminal rendering, TextMate grammars, and WebAssembly host integration.

## Grey Hack components

| Component | Responsibility |
|---|---|
| `greyhack-behaviour` | Shared path, address, password, naming, and service rules. |
| `greyhack-world` | Seeded computers, networks, file systems, services, economy, and persistent world state. |
| `greyhack-world-operations` | Shared-world operations and script-object lifetime management. |
| `greyhack-terminal-text` | Grey Hack rich text rendered as terminal output. |
| `greyhack-terminal-web` | Browser terminal layout, meshes, and WebGL rendering. |
| `greyhack-message-hook-client` | Client for the Grey Hack message-hook plugin. |
| `greybel/dotnet/greyhack-message-hook` | BepInEx plugin that connects external tools to Grey Hack. |
| `greybel/dotnet/greyhack-test-support` | Headless game and test-plugin infrastructure. |
| `greybel/dotnet/greyhack-util` | Contract, asset, and protocol extraction and generation. |

See [Grey Hack integration](greyhack-integration.md) and [Headless game testing](game-testing.md).

## Duplex

Duplex multiplexes versioned request/response and streaming protocols over a transport.

| Project | Responsibility |
|---|---|
| `duplex-protocol` | Parse definitions, check version compatibility, and build generator plans. |
| `duplex-core` | Channels, messages, streams, batching, routing, security, and connection lifetime. |
| `duplex-macros` | Generate Rust types, binary codecs, peers, and migrations. |
| `duplex-ts`, `duplex-cs` | Generate TypeScript and C# types and protocol APIs. |
| `duplex-driver-*` | In-memory, worker, stdio/TCP, HTTP, and WebSocket transports. |
| `duplex-tokio` | Shared Tokio scheduling, acceptance, and read gating. |
| `duplex-test-support` | Manual scheduling, in-memory transports, fixtures, and conformance services. |

The [duplex specification](../duplex/SPEC.md) defines runtime behavior. [Protocol definitions](../protocols/README.md) defines the declaration language and versioning rules. Language-specific usage is documented in [`duplex/ts`](../duplex/ts/README.md) and [`duplex/dotnet`](../duplex/dotnet/README.md).

## Rendering and assets

- `text-mesh/rs` implements TextMesh Pro-compatible rich text, layout, ANSI/HTML/canvas output, font data, and WebGL rendering.
- `text-mesh/shaders` contains the GLSL shader ports used by renderers.
- `text-mesh/unity` records reference behavior and fixtures from Unity.
- `assets/rs/asset-bundle` reads and writes compressed asset bundles and fingerprints.
- `assets/rs/greyhack-*-assets` embeds the game's terminal and world data.
- `assets/dotnet` provides the corresponding C# bundle support.

## Mono compatibility

`mono/rs` reproduces behavior that GreyScript inherits from Grey Hack's Mono runtime: number and string handling, seeded random numbers, dictionary ordering, and regular expressions. `mono/dotnet` records reference fixtures and provides the native Mono regex implementation.

## Shared utilities

- `common/rs/scheduler` supplies clocks, deferred tasks, and timers for Tokio, browsers, and tests.
- `common/rs/thin-str` provides compact shared immutable strings.
- `common/rs/vscode-uri` implements VS Code-compatible URI and path handling.
- [`common/ts`](../common/ts/README.md) contains shared TypeScript event, error, configuration, and WebAssembly helpers.

## Protocol and generated-code boundaries

Application protocols are declared in versioned `protocols/<name>/<major>.<minor>.protocol` files. Duplex's own wire layout is declared in `duplex/core`.

The definitions are the source of truth. Rust, TypeScript, and C# generators produce their types and codecs from the same versioned plans. Generated files are refreshed by their owning tests or build commands rather than edited independently.
