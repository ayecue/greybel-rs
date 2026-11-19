# greybel-builder-web

## 0.1.1 · 2026-11-19

### Patch changes

- greybel-core, greybel-transpiler: Chains of `+`, `-`, `*`, `/`, `%`, `^`, `and` and `or` operations of any length, like Grey Hack: a long `"..." + char(10) + "..."` expression no longer fails with "nesting exceeds the limit of 256 levels".
