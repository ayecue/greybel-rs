mod options;

use greybel_project::{DiagnosticSettings, SyntaxSettings};
use greybel_type_analyzer::InlayHintOptions;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    #[serde(deserialize_with = "Settings::extensions")]
    pub file_extensions: Vec<String>,
    pub formatter: bool,
    pub autocomplete: bool,
    pub hoverdocs: bool,
    pub diagnostic: bool,
    pub strict_mode: bool,
    pub syntax: SyntaxSettings,
    pub transpiler: TranspilerSettings,
    pub type_analyzer: TypeAnalyzerSettings,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranspilerSettings {
    pub beautify: BeautifySettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BeautifySettings {
    pub keep_parentheses: bool,
    pub indentation: IndentationKind,
    pub indentation_spaces: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum IndentationKind {
    #[default]
    Tab,
    Whitespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TypeAnalyzerSettings {
    pub exclude: String,
    pub diagnostics: DiagnosticSettings,
    pub inlay_hints: InlayHintSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InlayHintSettings {
    pub variable_types: bool,
    pub parameter_names: bool,
    pub return_types: bool,
}

impl Default for InlayHintSettings {
    fn default() -> Self {
        let options = InlayHintOptions::default();

        Self {
            variable_types: options.variable_types,
            parameter_names: options.parameter_names,
            return_types: options.return_types,
        }
    }
}

impl Default for TypeAnalyzerSettings {
    fn default() -> Self {
        Self {
            exclude: "**/{node_modules,.*}/**".to_owned(),
            diagnostics: DiagnosticSettings::default(),
            inlay_hints: InlayHintSettings::default(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            file_extensions: ["src", "gs", "ms"].map(str::to_owned).to_vec(),
            formatter: true,
            autocomplete: true,
            hoverdocs: true,
            diagnostic: true,
            strict_mode: false,
            syntax: SyntaxSettings::default(),
            transpiler: TranspilerSettings::default(),
            type_analyzer: TypeAnalyzerSettings::default(),
        }
    }
}

impl Default for BeautifySettings {
    fn default() -> Self {
        Self {
            keep_parentheses: false,
            indentation: IndentationKind::Tab,
            indentation_spaces: 2,
        }
    }
}

impl Settings {
    pub const SECTION: &str = "greybel";

    fn extensions<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ExtensionInput {
            Joined(String),
            List(Vec<String>),
        }

        let extensions = match ExtensionInput::deserialize(deserializer)? {
            ExtensionInput::Joined(comma_list) => {
                comma_list.split(',').map(str::to_owned).collect()
            }
            ExtensionInput::List(list) => list,
        };

        Ok(extensions
            .into_iter()
            .map(|extension| extension.trim().trim_start_matches('.').to_owned())
            .filter(|extension| !extension.is_empty())
            .collect())
    }
}
