//! Shared helpers for `greybel-bundler` integration tests.

#![allow(dead_code)]

pub mod installer;

use greybel_bundler::{AutoCompile, Build, BuildOptions, BuildOutput, Result};
use greybel_core::{Dialect, ParserOptions};
use greybel_documents::Uri;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_transpiler::TranspileOptions;

pub const MAIN: &str = "/project/src/main.src";

pub fn project() -> MemoryProvider {
    [
        (
            MAIN,
            "#include \"lib/util.src\"\nimport_code(\"lib/tool.src\")\nprint util + \"\"\"quoted\"\"\"\n",
        ),
        ("/project/src/lib/util.src", "util = \"x\"\n"),
        ("/project/src/lib/tool.src", "print \"tool\"\n"),
    ]
    .into_iter()
    .collect()
}

pub fn uri(path: &str) -> Uri {
    MemoryProvider::uri(path)
}

pub fn options() -> BuildOptions {
    BuildOptions {
        transpile: TranspileOptions {
            dialect: Dialect::GreyScript,
            ..TranspileOptions::default()
        },
        ..BuildOptions::default()
    }
}

/// Returns default build options with in-game auto-compilation configured to allow or disallow imports.
pub fn auto_compiled(allow_import: bool) -> BuildOptions {
    BuildOptions {
        auto_compile: Some(AutoCompile { allow_import }),
        ..options()
    }
}

pub fn build_with(
    options: BuildOptions,
    provider: &MemoryProvider,
    entries: &[&str],
) -> Result<BuildOutput> {
    let entries: Vec<Uri> = entries.iter().map(|path| uri(path)).collect();

    Build::new(options, &mut fastrand::Rng::with_seed(7)).run(&entries, provider)
}

pub fn build(options: BuildOptions) -> BuildOutput {
    build_with(options, &project(), &[MAIN]).expect("builds")
}

pub fn paths(output: &BuildOutput) -> Vec<&str> {
    output.files.iter().map(|file| file.path.as_str()).collect()
}

pub fn parse_errors(code: &str) -> Option<String> {
    greybel_core::parse(code, ParserOptions::new(Dialect::GreyScript))
        .err()
        .map(|error| error.to_string())
}
