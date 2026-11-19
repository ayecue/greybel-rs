# greybel-lsp-stdio

## 0.1.1 · 2026-11-19

### Patch changes

- greybel-core, greybel-interpreter, greybel-transpiler, greybel-type-analyzer: Chains of `+`, `-`, `*`, `/`, `%`, `^`, `and` and `or` operations of any length, like Grey Hack: a long `"..." + char(10) + "..."` expression no longer fails with "nesting exceeds the limit of 256 levels".

## 0.1.0 · 2026-11-18

- The first release of `greybel-lsp`, the GreyScript language server over stdio, for any editor with LSP support. It is the server greybel-vs runs.
- Diagnostics from the type analyzer, completion, hover, signature help, go to definition and type definition, references, rename, document and workspace symbols, code actions, semantic tokens, inlay hints, call hierarchy, folding and selection ranges, document colors and formatting.
- Settings are `greybel.*` through `workspace/configuration`; a project's `greybel.json` is read through the `custom/project` request.
