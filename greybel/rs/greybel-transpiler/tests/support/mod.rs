//! Shared fixture and snapshot helpers for `greybel-transpiler` integration tests.

#![allow(dead_code)]

pub mod emit_output;
pub mod greybel;
pub mod greyscript;
pub mod name_table;
pub mod path_mapping;
pub mod source_map;

use std::fmt::Write;
use std::path::{Path, PathBuf};

use greybel_transpiler::{
    BeautifyOptions, BuildMode, Indentation, Output, TranspileError, TranspileOptions,
};
use serde_json::Value;

const ENVIRONMENT: &[(&str, &str)] = &[("test", "\"foo\""), ("test2", "0.2"), ("TEST_ENV", "true")];

/// Non-default beautify settings: retain parentheses and indent with 10 spaces.
pub const BEAUTIFY_CUSTOM: BeautifyOptions = BeautifyOptions {
    retain_parentheses: true,
    indentation: Indentation::Spaces(10),
};

/// Path to one `greybel-transpiler/tests/fixtures/` subdirectory.
#[derive(Clone)]
pub struct Fixtures(PathBuf);

impl Fixtures {
    #[must_use]
    pub fn new(folder: &str) -> Self {
        Self(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join(folder),
        )
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }

    /// Builds reference-matching options with the test environment and intrinsics from `excluded-names.txt` excluded
    /// from renaming.
    #[must_use]
    pub fn options(&self, mode: BuildMode) -> TranspileOptions {
        let excluded =
            std::fs::read_to_string(self.0.join("excluded-names.txt")).expect("excluded names");

        TranspileOptions {
            mode,
            environment: ENVIRONMENT
                .iter()
                .map(|&(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
            excluded_names: excluded.lines().map(str::to_owned).collect(),
            ..TranspileOptions::default()
        }
    }

    /// Removes the fixture-root prefix from paths in `text` for stable snapshots.
    #[must_use]
    fn relative(&self, text: &str) -> String {
        text.replace(&format!("{}/", self.0.display()), "")
    }

    /// Appends a titled output section, or an error section with fixture-relative paths.
    fn section(&self, out: &mut String, title: &str, result: Result<String, TranspileError>) {
        let body =
            result.unwrap_or_else(|error| format!("error: {}", self.relative(&error.to_string())));

        writeln!(out, "=== {title}\n{body}").expect("writing to a String");
    }

    /// Runs every case and concatenates its result under its title.
    pub fn sections<T>(
        &self,
        cases: impl IntoIterator<Item = (impl AsRef<str>, T)>,
        mut run: impl FnMut(T) -> Result<String, TranspileError>,
    ) -> String {
        let mut out = String::new();

        for (title, case) in cases {
            self.section(&mut out, title.as_ref(), run(case));
        }

        out
    }

    /// Formats the entry output first and prefixes every remaining output with its fixture-relative target path.
    #[must_use]
    pub fn outputs(&self, outputs: Vec<Output>) -> String {
        outputs
            .iter()
            .enumerate()
            .map(|(index, output)| match index {
                0 => output.code.clone(),
                _ => format!(
                    "--- {}\n{}",
                    self.relative(&output.target.label()),
                    output.code
                ),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Returns `options` with obfuscation disabled.
pub fn unobfuscated(options: TranspileOptions) -> TranspileOptions {
    TranspileOptions {
        obfuscation: false,
        ..options
    }
}

pub fn strings(value: &Value) -> Vec<Option<String>> {
    value
        .as_array()
        .expect("array")
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}
