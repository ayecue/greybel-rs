# greybel-rs

Greybel is a toolchain for [MiniScript](https://miniscript.org/) and GreyScript, the scripting language used by [Grey Hack](https://greyhackgame.com). It includes a parser, transpiler, bundler, interpreter, type analyzer, language server, and debugger.

This repository contains the Rust implementation and its TypeScript and .NET applications. The main tools are:

- **`greybel` CLI:** build, transform, import, execute, and debug scripts; start a REPL, web editor, or MCP server.
- **VS Code extension:** syntax highlighting, diagnostics, completion, hover documentation, formatting, builds, and debugging.
- **Language server:** GreyScript and MiniScript editing features for LSP-compatible editors.
- **Web tools:** a browser editor and searchable API reference.
- **Grey Hack integration:** import and run scripts in the game through the message hook, with a setup wizard for installation.

## Install the CLI

Download a platform binary from [GitHub Releases](https://github.com/ayecue/greybel-rs/releases), or install from source with Rust:

```sh
cargo install --path apps/rs/greybel-cli --locked
```

Run `greybel --help` for the complete command list and `greybel <command> --help` for command options.

## Quick start

Build a script and its dependencies:

```sh
greybel build path/to/script.src
```

Execute a script in the local mock environment:

```sh
greybel execute path/to/script.src
```

Start an interactive session:

```sh
greybel repl
```

The [CLI guide](docs/cli.md) covers project configuration, output modes, imports, installers, debugging, and in-game execution.

## Editor setup

Install [Greybel from the VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=ayecue.greybel-vs) or [Open VSX](https://open-vsx.org/extension/ayecue/greybel-vs). The extension supports `.src`, `.gs`, and `.ms` files.

For another editor, install the language server from its platform release or from source:

```sh
cargo install --path apps/rs/greybel-lsp-stdio --locked
```

Then configure the editor's LSP client to run:

```sh
greybel-lsp --stdio
```

See the [VS Code guide](docs/vscode.md) for editor features and project settings. The [web editor](https://editor.greyscript.org) and [GreyScript API reference](https://documentation.greyscript.org) are also available online.

## Grey Hack integration

The message hook enables in-game imports, builds, execution, and debugging. The setup wizard can install BepInEx and the plugin for you.

See [Grey Hack integration](docs/greyhack-integration.md) for setup and troubleshooting. Local execution does not require Grey Hack or the plugin.

## Repository layout

Each language workspace builds independently. Path dependencies connect the workspaces.

| Folder | Contents |
|---|---|
| [`apps/`](apps) | CLI and server binaries, VS Code extension, web applications, and setup wizard. |
| [`greybel/`](greybel) | Language libraries, runtimes, intrinsics, editor services, and Grey Hack integrations. |
| [`duplex/`](duplex) | Versioned typed messaging, transport drivers, generators, and conformance tests. |
| [`protocols/`](protocols) | Protocol definitions used by Greybel applications and services. |
| [`text-mesh/`](text-mesh) | TextMesh Pro-compatible parsing, layout, and rendering. |
| [`assets/`](assets) | Asset-bundle support and embedded assets. |
| [`mono/`](mono) | Grey Hack-compatible Mono number, string, collection, random, and regex behavior. |
| [`common/`](common) | Shared utilities. |
| [`tools/`](tools) | Repository tooling, including the reman release and task manager. |

Library dependencies point toward the supporting workspaces. The VS Code extension also packages the Rust application binaries it needs. See [Architecture](docs/architecture.md) for component responsibilities and dependency boundaries.

## Documentation

- [CLI guide](docs/cli.md)
- [VS Code guide](docs/vscode.md)
- [Grey Hack integration](docs/greyhack-integration.md)
- [Architecture](docs/architecture.md)
- [Development](docs/development.md)
- [Testing](docs/testing.md)
- [Headless game testing](docs/game-testing.md)
- [GreyScript emulation](greybel/rs/greyscript-intrinsics/README.md)
- [Protocol definitions](protocols/README.md)
- [Duplex specification](duplex/SPEC.md)
- [reman task and release manager](tools/reman/README.md)

## Development

Install Rust, Node.js with pnpm, and the .NET SDK for the components you work on. Then run the repository checks through `make`:

```sh
make install
make lint
make test
```

Run `make help` to list language-specific and integration targets. See [Development](docs/development.md) for formatting and releases, and [Testing](docs/testing.md) for test conventions and specialized test environments.

## License

MIT
