---
rs/greybel-core: patch
rs/greybel-transpiler: patch
rs/greybel-interpreter: patch
rs/greybel-type-analyzer: patch
rs/greybel-api-reference: none
---

Chains of `+`, `-`, `*`, `/`, `%`, `^`, `and` and `or` operations of any length, like Grey Hack: a long `"..." + char(10) + "..."` expression no longer fails with "nesting exceeds the limit of 256 levels".
