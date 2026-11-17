use std::collections::HashMap;

use greybel_core::{Dialect, ParserOptions, SyntaxFeatures};
use greybel_documents::{DocumentOptions, NativeImportMode, PathResolution, Uri};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BuildMode {
    #[default]
    Default,
    Uglify(UglifyOptions),
    Beautify(BeautifyOptions),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UglifyOptions {
    pub disable_literals_optimization: bool,
    pub disable_namespaces_optimization: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BeautifyOptions {
    pub retain_parentheses: bool,
    pub indentation: Indentation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MultilineStringStyle {
    #[default]
    Concatenation,
    Join,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Indentation {
    #[default]
    Tab,
    Spaces(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranspileOptions {
    pub dialect: Dialect,
    pub mode: BuildMode,
    pub dev_mode: bool,
    pub obfuscation: bool,
    pub environment: HashMap<String, String>,
    pub strict_mode: bool,
    pub syntax_features: SyntaxFeatures,
    pub excluded_names: Vec<String>,
    pub native_functions: Vec<String>,
    pub path_mapping: Option<PathMapping>,
    pub retain_debugger: bool,
    pub retain_multiline_strings: bool,
    pub multiline_strings: MultilineStringStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMapping {
    pub local_root: Uri,
    pub ingame_directory: String,
}

impl PathMapping {
    #[must_use]
    pub fn map(&self, uri: &Uri) -> String {
        match self.relative(uri) {
            Some(relative) => format!("{}/{relative}", self.ingame_directory.trim_end_matches('/')),
            None => uri.label(),
        }
    }

    #[must_use]
    pub fn relative(&self, uri: &Uri) -> Option<String> {
        PathResolution::path_in(&self.local_root, uri)
            .filter(|path| !path.is_empty())
            .map(ToOwned::to_owned)
    }
}

impl TranspileOptions {
    #[must_use]
    pub fn load_options(&self) -> DocumentOptions {
        DocumentOptions {
            parser: ParserOptions {
                environment: self.environment.clone(),
                strict_mode: self.strict_mode,
                syntax_features: self.syntax_features,
                preserve: true,
                mono_number_parsing: false,
                ..ParserOptions::new(self.dialect)
            },
            unsafe_mode: false,
            native_imports: NativeImportMode::Emission,
        }
    }

    #[must_use]
    pub fn aliases_natives(&self) -> bool {
        matches!(self.mode, BuildMode::Uglify(uglify) if !uglify.disable_namespaces_optimization)
    }

    #[must_use]
    pub fn shortens_assignments(&self) -> bool {
        match self.dialect {
            Dialect::GreyScript => self.dev_mode && !self.strict_mode,
            Dialect::MiniScript | Dialect::Greybel => !self.strict_mode,
        }
    }
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            dialect: Dialect::Greybel,
            mode: BuildMode::Default,
            dev_mode: false,
            obfuscation: true,
            environment: HashMap::new(),
            strict_mode: false,
            syntax_features: SyntaxFeatures::ALL,
            excluded_names: Vec::new(),
            native_functions: Vec::new(),
            path_mapping: None,
            retain_debugger: false,
            retain_multiline_strings: false,
            multiline_strings: MultilineStringStyle::Concatenation,
        }
    }
}
