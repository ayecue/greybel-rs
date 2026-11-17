use greybel_core::Dialect;
use greybel_documents::{DocumentOptions, PathResolution, Uri};
use greybel_transpiler::{BeautifyOptions, Indentation};
use greybel_type_analyzer::{Analyzer, DiagnosticOptions, InlayHintOptions};

use super::{IndentationKind, Settings};

impl Settings {
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
        options.parser.syntax_features = self.syntax.features();

        options
    }

    #[must_use]
    pub fn diagnostic_options(&self) -> DiagnosticOptions {
        self.type_analyzer.diagnostics.options(self.strict_mode)
    }

    #[must_use]
    pub fn inlay_hint_options(&self) -> InlayHintOptions {
        let hints = self.type_analyzer.inlay_hints;

        InlayHintOptions {
            variable_types: hints.variable_types,
            parameter_names: hints.parameter_names,
            parameter_types: hints.parameter_types,
            return_types: hints.return_types,
        }
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
