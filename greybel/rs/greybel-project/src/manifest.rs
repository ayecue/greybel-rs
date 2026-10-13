use std::collections::BTreeMap;

use greybel_bundler::{BuildOptions, Installer as Installers};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use crate::profile::Profile;

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
    /// Project-relative glob for files excluded from type analysis.
    #[schemars(extend("x-vscode-setting" = "greybel.typeAnalyzer.exclude"))]
    pub exclude: String,
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
            exclude: Self::DEFAULT_EXCLUDE.to_owned(),
            format: Format::default(),
            build: BuildSettings::default(),
            run: RunSettings::default(),
            profiles: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Format {
    /// Preserves all source parentheses.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.beautify.keepParentheses"))]
    pub retain_parentheses: bool,
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.beautify.indentation"))]
    pub indentation: Indentation,
    /// Spaces per indentation level when using `Whitespace`.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.beautify.indentationSpaces"))]
    pub indentation_spaces: u32,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            retain_parentheses: false,
            indentation: Indentation::Tab,
            indentation_spaces: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Indentation {
    #[default]
    Tab,
    Whitespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct BuildSettings {
    /// Project-relative output folder for builds.
    pub output: String,
    /// Output entry filename for single-file builds; empty means no name.
    #[serde(deserialize_with = "BuildSettings::output_name")]
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.outputFilename"))]
    pub output_name: Option<String>,
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.buildType"))]
    pub mode: BuildMode,
    /// Also shortens names accessed as strings, including map keys.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.obfuscation"))]
    pub obfuscation: bool,
    /// Preserves repeated literals during uglification.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.dlo"))]
    pub disable_literals_optimization: bool,
    /// Preserves names during uglification.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.dno"))]
    pub disable_namespaces_optimization: bool,
    /// Names excluded from uglification.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.excludedNamespaces"))]
    pub excluded_namespaces: Vec<String>,
    /// In-game folder from which the build runs.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.ingameDirectory"))]
    pub ingame_directory: String,
    /// Compiles the build in-game and removes its source files.
    #[schemars(extend("x-vscode-setting" = "greybel.autoCompile"))]
    pub auto_compile: bool,
    pub installer: InstallerSettings,
    pub deploy: DeploySettings,
    /// Build-time values for `#envar`.
    #[schemars(extend(
        "x-vscode-setting" = {
            "files": "greybel.transpiler.environmentFile",
            "values": "greybel.transpiler.environmentVariables"
        }
    ))]
    pub variables: VariableSettings,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            output: "build".to_owned(),
            output_name: None,
            mode: BuildMode::Default,
            obfuscation: false,
            disable_literals_optimization: false,
            disable_namespaces_optimization: false,
            excluded_namespaces: Vec::new(),
            ingame_directory: BuildOptions::DEFAULT_INGAME_DIRECTORY.to_owned(),
            auto_compile: true,
            installer: InstallerSettings::default(),
            deploy: DeploySettings::default(),
            variables: VariableSettings::default(),
        }
    }
}

impl BuildSettings {
    fn output_name<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
        let name = Option::<String>::deserialize(deserializer)?;

        Ok(name.filter(|name| !name.trim().is_empty()))
    }
}

/// Build output mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum BuildMode {
    /// Original formatting without comments.
    #[default]
    Default,
    /// Minified output.
    Uglify,
    /// Output formatted according to `format`.
    Beautify,
}

/// Installer files that recreate the build when run in-game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct InstallerSettings {
    /// Enables installer-file output.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.installer.active"))]
    pub active: bool,
    /// Maximum characters per installer file; longer installers are split.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.installer.maxChars"))]
    pub max_chars: u32,
    /// Allows installer-compiled programs to import code at runtime.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.installer.allowImport"))]
    pub allow_import: bool,
}

impl Default for InstallerSettings {
    fn default() -> Self {
        Self {
            active: true,
            max_chars: u32::try_from(Installers::DEFAULT_MAX_CHARS)
                .expect("the installer's default length fits in u32"),
            allow_import: false,
        }
    }
}

/// In-game builds performed through the message hook.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct DeploySettings {
    /// Allows game-compiled programs to import code at runtime.
    #[schemars(extend("x-vscode-setting" = "greybel.createIngame.allowImport"))]
    pub allow_import: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct RunSettings {
    /// Seed text for simulated-world generation; identical seeds produce identical worlds.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.seed"))]
    pub seed: String,
    /// Values for the program's `params`.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.defaultArgs", "x-vscode-format" = "spaceList"))]
    pub params: Vec<String>,
    /// Binary name used when building into the user's home folder so the program can launch itself. The in-game
    /// binary exists only for the run's duration.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.programName"))]
    pub program_name: String,
    /// Simulated-program memory limit in MiB before interruption, matching Grey Hack's out-of-memory behavior.
    /// Call nesting is unlimited.
    #[schemars(extend("x-vscode-setting" = "greybel.interpreter.memoryLimit"))]
    pub memory_limit: u32,
    /// Run-time values for `#envar`.
    #[schemars(extend(
        "x-vscode-setting" = {
            "files": "greybel.interpreter.environmentFile",
            "values": "greybel.interpreter.environmentVariables"
        }
    ))]
    pub variables: VariableSettings,
}

impl RunSettings {
    pub const DEFAULT_MEMORY_LIMIT: u32 = 1024;
    pub const DEFAULT_PROGRAM_NAME: &str = "myprogram";
    pub const DEFAULT_SEED: &str = "test";
}

impl Default for RunSettings {
    fn default() -> Self {
        Self {
            seed: Self::DEFAULT_SEED.to_owned(),
            params: Vec::new(),
            program_name: Self::DEFAULT_PROGRAM_NAME.to_owned(),
            memory_limit: Self::DEFAULT_MEMORY_LIMIT,
            variables: VariableSettings::default(),
        }
    }
}

/// Values for `#envar`, loaded from `NAME=value` lines in `files` and then `values`; later entries override
/// earlier entries with the same name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct VariableSettings {
    /// Project-relative files containing `NAME=value` lines; missing files contribute no values.
    #[schemars(extend("x-vscode-format" = "commaList"))]
    pub files: Vec<String>,
    pub values: BTreeMap<String, String>,
}

impl VariableSettings {
    pub const DEFAULT_FILE: &str = ".env";
}

impl Default for VariableSettings {
    fn default() -> Self {
        Self {
            files: vec![Self::DEFAULT_FILE.to_owned()],
            values: BTreeMap::new(),
        }
    }
}
