# greybel-editor-web

## 0.1.1 · 2026-11-19

### Patch changes

- ts/greybel-api-reference-web: The guide has a Documentation comments chapter (doc-comment tags, `@ignore`, diagnostic settings) and describes `#include type`, `//include` and `//import`. The web editor and the API reference site show the Greybel logo and favicon.
- greybel-core, greybel-interpreter, greybel-transpiler, greybel-type-analyzer: Chains of `+`, `-`, `*`, `/`, `%`, `^`, `and` and `or` operations of any length, like Grey Hack: a long `"..." + char(10) + "..."` expression no longer fails with "nesting exceeds the limit of 256 levels".

## 0.1.0 · 2026-11-18

- The first version of the web editor on greybel-rs: multi-file projects with a `greybel.json`, language-server features, runs and debugging in terminals that share one simulated world, transforms, the API reference, resizable panels, and saving projects to greybel-editor-service.
