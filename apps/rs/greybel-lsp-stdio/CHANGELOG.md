# greybel-lsp-stdio

## 0.1.0 · 2026-11-18

- The first release of `greybel-lsp`, the GreyScript language server over stdio, for any editor with LSP support. It is the server greybel-vs runs.
- Diagnostics from the type analyzer, completion, hover, signature help, go to definition and type definition, references, rename, document and workspace symbols, code actions, semantic tokens, inlay hints, call hierarchy, folding and selection ranges, document colors and formatting.
- Settings are `greybel.*` through `workspace/configuration`; a project's `greybel.json` is read through the `custom/project` request.
