# greybel-cli

## 0.1.0 · 2026-11-18

- The first release of `greybel`, the GreyScript command-line tool written in Rust: `build` (transpile with imports, installers and in-game builds), `import`, `execute` in a simulated world or in Grey Hack through the message hook, `repl`, `ui` (the web editor) and `mcp`.
- Projects are configured in `greybel.json`, with profiles, `#envar` variables, syntax features, diagnostic levels and terminal line limits.
- `greybel mcp` serves the tools `check`, `build`, `run`, `transform`, `import`, `api_search` and `api_reference`, and code navigation by symbol name: `document_symbols`, `find_symbol`, `find_references`, `describe`, `call_hierarchy` and `rename`.
