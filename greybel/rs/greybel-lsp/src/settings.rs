use greybel_core::Dialect;
use greybel_documents::{DocumentOptions, PathResolution, Uri};
use greybel_transpiler::{BeautifyOptions, Indentation};
use greybel_type_analyzer::{Analyzer, DiagnosticKind, DiagnosticOptions};
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
}

impl Default for TypeAnalyzerSettings {
    fn default() -> Self {
        Self {
            exclude: "**/{node_modules,.*}/**".to_owned(),
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

    #[must_use]
    pub fn file_pattern(&self) -> String {
        match self.file_extensions.as_slice() {
            [extension] => format!("**/*.{extension}"),
            extensions => format!("**/*.{{{}}}", extensions.join(",")),
        }
    }

    #[must_use]
    pub fn resolution(&self, folders: &[Uri]) -> PathResolution {
        PathResolution::new(self.file_extensions.iter().cloned())
            .with_folders(folders.iter().cloned())
    }

    #[must_use]
    pub fn document_options(&self, dialect: Dialect) -> DocumentOptions {
        let mut options = Analyzer::document_options(dialect);

        options.parser.strict_mode = self.strict_mode;

        options
    }

    #[must_use]
    pub fn diagnostic_options(&self) -> DiagnosticOptions {
        let mut options = DiagnosticOptions::default();

        if !self.strict_mode {
            options
                .disabled_kinds
                .insert(DiagnosticKind::CallSignatureMismatch);
        }

        options
    }

    #[must_use]
    pub fn beautify_options(&self) -> BeautifyOptions {
        let beautify = &self.transpiler.beautify;

        BeautifyOptions {
            retain_parentheses: beautify.keep_parentheses,
            indentation: match beautify.indentation {
                IndentationKind::Tab => Indentation::Tab,
                IndentationKind::Whitespace => Indentation::Spaces(beautify.indentation_spaces),
            },
        }
    }
}
