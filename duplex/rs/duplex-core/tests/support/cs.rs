//! Helpers for .NET core files generated from the definitions.

use std::path::Path;

use duplex_test_support::generated::GeneratedFolder;

/// Returns the generated-file folder for `duplex/dotnet/duplex-core`.
pub fn csharp() -> GeneratedFolder {
    GeneratedFolder::csharp(
        Path::new(env!("DUPLEX_DOTNET")).join("duplex-core"),
        "duplex-core",
    )
}
