# mono-fixtures

Records behavior of Grey Hack's own .NET runtime as fixtures and generated
tables for `mono-number`, `mono-string`, and `mono-dictionary`.

```sh
./run.sh
# Or, from mono/dotnet:
make fixtures
```

Requirements and overrides:

| Variable | Purpose | Default |
|---|---|---|
| `GREY_HACK_PATH` | macOS Grey Hack installation | `~/Library/Application Support/Steam/steamapps/common/Grey Hack` |
| `DOTNET` | .NET SDK executable | `dotnet` |
| `CC` | C compiler for the native host | `cc` |

The program targets .NET Framework 4.7.2 and runs on the game's
`libmonobdwgc-2.0.dylib` with its `mscorlib.dll`. Do not substitute system
Mono: behavior differs, including `String.GetHashCode`.

`run.sh` builds the managed program and native host, then creates a temporary
runtime layout containing the game's `mscorlib`, `System`, and `System.Core`
assemblies. A `lib` symlink to the game's `Frameworks` lets corlib resolve
`libmono-native` from `<managed dir>/../lib`. The host also receives the game's
`MonoBleedingEdge/etc` folder.

All recording uses the invariant culture, matching the culture established by
the game before script execution. The recorded sets cover:

- number formatting, parsing, rounding, and hashing;
- UTF-16 case tables, string operations, comparison, and collation data;
- `Dictionary<int, int>` enumeration order across deterministic edits and
  resizes.

Output is deterministic: rerunning without a game update should produce no
file changes. To add a fixture set, implement `IFixtureBuilder` and register it
in `src/Program.cs`.
