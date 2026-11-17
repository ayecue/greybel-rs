mod build;
mod diagnostics;
mod profile;
mod run;
mod schema;
mod syntax;
mod variables;

use std::collections::BTreeMap;

pub use build::{
    BuildMode, BuildSettings, DeploySettings, Format, Indentation, InstallerSettings,
    MultilineStringStyle,
};
pub use diagnostics::{DiagnosticLevel, DiagnosticSettings};
pub use profile::{Profile, ProfileError};
pub use run::{LineLimits, RunSettings};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
pub use syntax::SyntaxSettings;
pub use variables::{VariableFile, VariableSettings};

/// A project's `greybel.json`; omitted settings use defaults. `x-vscode-setting` identifies the synced greybel-vs
/// setting. `x-vscode-format` controls list serialization: `commaList` and `spaceList` join items, while `first`
/// selects the first item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
#[schemars(title = "greybel.json", rename = "Manifest")]
pub struct Manifest {
    /// Editor-facing JSON Schema for this file.
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Entry files for builds and runs, relative to the project folder.
    #[schemars(extend("x-vscode-setting" = "greybel.rootFile", "x-vscode-format" = "first"))]
    pub entries: Vec<String>,
    /// Optional import extensions in resolution order; leading dots are removed.
    #[serde(deserialize_with = "Manifest::extensions")]
    #[schemars(extend("x-vscode-setting" = "greybel.fileExtensions", "x-vscode-format" = "commaList"))]
    pub extensions: Vec<String>,
    /// Restricts syntax to Grey Hack's compiler (MiniScript 1.6.2 for MiniScript files), excluding Greybel extensions
    /// such as `#include`, `&`, and `+=`.
    #[schemars(extend("x-vscode-setting" = "greybel.strictMode"))]
    pub strict_mode: bool,
    /// Greybel syntax features turned on or off by name, e.g. `{"ternary": false}`; a feature not listed is on.
    /// Strict mode turns all of them off.
    #[schemars(extend("x-vscode-setting" = "greybel.syntax"))]
    pub syntax: SyntaxSettings,
    /// Project-relative glob for files excluded from type analysis.
    #[schemars(extend("x-vscode-setting" = "greybel.typeAnalyzer.exclude"))]
    pub exclude: String,
    /// Diagnostic levels by diagnostic code: `off` drops a diagnostic, a severity replaces its default.
    #[schemars(extend("x-vscode-setting" = "greybel.typeAnalyzer.diagnostics"))]
    pub diagnostics: DiagnosticSettings,
    /// Formatting used by the editor and beautified builds.
    pub format: Format,
    pub build: BuildSettings,
    pub run: RunSettings,
    /// Named `build` and `run` overrides selected by CLI `--profile`, MCP tool `profile`, or greybel-vs
    /// `greybel.profile`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub profiles: BTreeMap<String, Profile>,
}

impl Manifest {
    pub const DEFAULT_EXCLUDE: &str = "**/{node_modules,.*}/**";
    /// Default optional import extensions.
    pub const DEFAULT_EXTENSIONS: [&str; 3] = ["src", "gs", "ms"];

    /// Extensions normalized by trimming whitespace and leading dots and removing empty entries.
    fn extensions<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
        let extensions = Vec::<String>::deserialize(deserializer)?;

        Ok(extensions
            .iter()
            .map(|extension| extension.trim().trim_start_matches('.').to_owned())
            .filter(|extension| !extension.is_empty())
            .collect())
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            schema: None,
            entries: Vec::new(),
            extensions: Self::DEFAULT_EXTENSIONS.map(str::to_owned).to_vec(),
            strict_mode: false,
            syntax: SyntaxSettings::default(),
            exclude: Self::DEFAULT_EXCLUDE.to_owned(),
            diagnostics: DiagnosticSettings::default(),
            format: Format::default(),
            build: BuildSettings::default(),
            run: RunSettings::default(),
            profiles: BTreeMap::new(),
        }
    }
}
