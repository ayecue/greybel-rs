use greybel_bundler::{BuildOptions, Installer as Installers};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

use super::VariableSettings;

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
    /// How builds write strings with line breaks, which Grey Hack does not accept.
    #[schemars(extend("x-vscode-setting" = "greybel.transpiler.multilineStrings"))]
    pub multiline_strings: MultilineStringStyle,
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
            multiline_strings: MultilineStringStyle::Concatenation,
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

/// How builds write a string with line breaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum MultilineStringStyle {
    /// `"a"+__NL+"b"` with `__NL=char(10)`; slow for many lines, as each `+` copies the string built so far.
    #[default]
    Concatenation,
    /// `__MSTR(["a","b"])`, a function that joins the lines with `char(10)` in one pass.
    Join,
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
