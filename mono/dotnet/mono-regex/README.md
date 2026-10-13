# mono-regex

Builds a Native AOT shared library that runs the Mono corefx regex engine used
by Grey Hack. The Rust `mono-regex` crate links it with the `dotnet` feature.

```sh
./build.sh                 # host platform
./build.sh all             # all supported desktop RIDs
./build.sh linux-x64       # one or more explicit RIDs
# Or from mono/dotnet:
make regex
make regex RIDS=all
```

A .NET 10 SDK is required. Libraries are written to
`dist/<rid>/libMonoRegex.{dylib,so}` or `dist/<rid>/MonoRegex.dll`. The crate's
`build.rs` locates the RID-specific directory through `MONO_REGEX_DIST`, which
the repository's `.cargo/config.toml` sets; `MONO_REGEX_LIB_DIR` overrides it
with a direct library directory.

From `mono/rs`:

```sh
cargo test -p mono-regex --features dotnet
```

Use Microsoft's .NET SDK on macOS. Homebrew's source-built `dotnet` cannot link
`osx-arm64` in this configuration because it requests OpenSSL; for example:

```sh
DOTNET=/usr/local/share/dotnet/dotnet ./build.sh
```

## C ABI

| Export | Operation |
|---|---|
| `mono_regex_is_match` | Tests input against a pattern. |
| `mono_regex_matches` | Returns all matches. |
| `mono_regex_split` | Splits input around matches. |
| `mono_regex_replace` | Replaces matches using a replacement string. |
| `mono_regex_free` | Releases a result's native allocations and zeroes it. |

Each operation accepts UTF-16 pointers with explicit `int` lengths,
`RegexOptions` as an integer, a timeout in milliseconds (`-1` means
`Regex.InfiniteMatchTimeout`), and a caller-provided `Result*`. Replacement
also takes a UTF-16 replacement pointer and length.

`Result` has sequential C layout with these fields in order:

| Field | Meaning |
|---|---|
| `Status: int` | `0` success, `1` timeout, `2` failure. |
| `IsMatch: int` | Nonzero when `is_match` matched. |
| `Text`, `TextLength` | Replacement text, concatenated split parts or match values, or the failure message. |
| `Numbers`, `NumbersLength` | One length per split part, or alternating match index and length values. |

The library owns `Text` and `Numbers`. The caller must eventually pass every
returned `Result` to `mono_regex_free`; the pointers remain valid until then.
Lengths count UTF-16 code units or integers, not bytes.

## Vendored engine

`corefx/` contains mono/corefx regex sources at commit `c4eeab9`, matching the
engine lineage shipped by Grey Hack, plus their required common helpers. The
sources are MIT-licensed; see `corefx/LICENSE.TXT`.

Local changes are limited to the `MonoCorefx.Text.RegularExpressions`
namespace and `SR.cs` resource constants. The constants preserve message text
from the game's `System.dll`, including its quoting, backslashes, and shorter
default-timeout message. The project disables nullable checks and suppresses
warnings from compiling these older sources with a modern compiler.

## Cross-compilation and compatibility

Cross-OS targets use Zig through the `StuDev.AotAnywhere` MSBuild SDK, which
downloads Zig. Same-OS targets use the .NET SDK linker because Zig-linked macOS
libraries abort when the runtime throws an exception.

`InvariantGlobalization` removes the ICU dependency. Consequently,
case-insensitive matching uses modern .NET's invariant casing rather than
Mono's `TextInfo` and can differ from the game for some non-ASCII letters.
