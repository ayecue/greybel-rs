# Asset bundle .NET project

[`asset-bundle`](asset-bundle/) implements the bundle and fingerprint formats
shared with the Rust `asset-bundle` crate.

```sh
dotnet test --solution asset-bundle/AssetBundle.slnx
```

## Format

Bundles are little-endian and contain, in order:

1. Magic `BNDL` and format version 1 as one byte.
2. File count as `u32`.
3. For each file: UTF-8 name length as `u16`, name bytes, unpacked size as
   `u32`, and packed size as `u32`.
4. Each file's raw DEFLATE stream in the same order. Every stream is complete;
   an empty file uses the empty final block `03 00`.

`fingerprint.json` records the game version and the SHA-256 of each file's
unpacked contents, sorted by name.

The .NET tests read `rust.bundle` and verify the shared fingerprint. The Rust
tests read both `rust.bundle` and `dotnet.bundle`. `Directory.Build.props`
defines `AssetsRust` so the projects can locate the Rust workspace.
