# greybel-cli

## 0.2.0 · 2026-11-19

### Minor changes

- The setup wizard is a small Rust program, `greyhack-setup` (about 5 MB, the .NET one was about 23 MB), with a window and the same command line; `greybel setup` and the VS Code command **Greybel: Set up Grey Hack** use the same steps. It installs the message hook release of its own commit, shows the Steam launch option macOS and Linux need, and removes the duplicate `libsteam_api.dylib` that crashes the game under BepInEx on macOS. An import, build or run that can't reach the game offers the setup in VS Code. `greybel update` replaces the CLI with its newest release from DigitalOcean Spaces, and commands in a terminal mention a newer version once a day.

### Patch changes

- greybel-core, greybel-interpreter, greybel-transpiler, greybel-type-analyzer: Chains of `+`, `-`, `*`, `/`, `%`, `^`, `and` and `or` operations of any length, like Grey Hack: a long `"..." + char(10) + "..."` expression no longer fails with "nesting exceeds the limit of 256 levels".

## 0.1.0 · 2026-11-18

- The first release of `greybel`, the GreyScript command-line tool written in Rust: `build` (transpile with imports, installers and in-game builds), `import`, `execute` in a simulated world or in Grey Hack through the message hook, `repl`, `ui` (the web editor) and `mcp`.
- Projects are configured in `greybel.json`, with profiles, `#envar` variables, syntax features, diagnostic levels and terminal line limits.
- `greybel mcp` serves the tools `check`, `build`, `run`, `transform`, `import`, `api_search` and `api_reference`, and code navigation by symbol name: `document_symbols`, `find_symbol`, `find_references`, `describe`, `call_hierarchy` and `rename`.
- `execute`, `repl` and `mcp` run programs in the simulated world server that the commands and VS Code windows of a machine share (`--world-port`, `--world-idle-timeout`, `--own-world`, `--world`), started in the background when none runs. `greybel world serve` and `greybel world reset` serve and reset it; worlds are saved per seed in `greybel/worlds` of the user's data folder (`--worlds`, `GREYBEL_WORLDS`).
- `--user root` runs simulated programs as `root`.
