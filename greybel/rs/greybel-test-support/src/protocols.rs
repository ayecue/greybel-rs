//! Loads checked-in protocol definitions and generated TypeScript/C# outputs for tests.

use std::path::Path;

use duplex_test_support::generated::{self, GeneratedFolder, MinorVersionTable};

/// Protocol-module import specifier in generated TypeScript role modules.
pub const PROTOCOL: &str = "./protocol.generated.js";

/// Loads definition `name`'s minor-version table for `major` from `PROTOCOLS`.
#[must_use]
pub fn major(name: &str, major: u16) -> MinorVersionTable<'static> {
    generated::major(Path::new(env!("PROTOCOLS")).join(name), major)
}

/// Creates a generated-folder checker for `greybel/ts/<package>`, owned by crate `package`.
#[must_use]
pub fn typescript(package: &'static str) -> GeneratedFolder {
    GeneratedFolder::typescript(Path::new(env!("GREYBEL_TS")).join(package), package)
}

/// Creates a generated-folder checker for `greybel/dotnet/<folder>`, owned by crate `package`.
#[must_use]
pub fn csharp(folder: &str, package: &'static str) -> GeneratedFolder {
    GeneratedFolder::csharp(Path::new(env!("GREYBEL_DOTNET")).join(folder), package)
}
