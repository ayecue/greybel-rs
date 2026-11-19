# greybel-api-reference-web

## 0.1.1 · 2026-11-19

### Patch changes

- The guide has a Documentation comments chapter (doc-comment tags, `@ignore`, diagnostic settings) and describes `#include type`, `//include` and `//import`. The web editor and the API reference site show the Greybel logo and favicon.
- greybel-core: Chains of `+`, `-`, `*`, `/`, `%`, `^`, `and` and `or` operations of any length, like Grey Hack: a long `"..." + char(10) + "..."` expression no longer fails with "nesting exceeds the limit of 256 levels".
