use gen_lsp_types as lsp;
use greybel_type_analyzer::{Diagnostic, DiagnosticKind, Severity};

use super::ToLsp;

const SOURCE: &str = "greybel";

impl ToLsp for Diagnostic {
    type Output = lsp::Diagnostic;

    fn to_lsp(&self) -> lsp::Diagnostic {
        lsp::Diagnostic {
            range: self.range.to_lsp(),
            severity: Some(self.severity.to_lsp()),
            code: Some(lsp::Code::String(self.kind.code().to_owned())),
            source: Some(SOURCE.to_owned()),
            message: lsp::Message::String(self.message.clone()),
            tags: self.kind.to_lsp().map(|tag| vec![tag]),
            ..lsp::Diagnostic::default()
        }
    }
}

/// Deprecated uses show struck through; unused and unreachable code shows faded.
impl ToLsp for DiagnosticKind {
    type Output = Option<lsp::DiagnosticTag>;

    fn to_lsp(&self) -> Option<lsp::DiagnosticTag> {
        match self {
            DiagnosticKind::Deprecated => Some(lsp::DiagnosticTag::Deprecated),
            DiagnosticKind::UnusedLocal
            | DiagnosticKind::UnusedParameter
            | DiagnosticKind::DeadCodeAfterTerminator => Some(lsp::DiagnosticTag::Unnecessary),
            _ => None,
        }
    }
}

impl ToLsp for Severity {
    type Output = lsp::DiagnosticSeverity;

    fn to_lsp(&self) -> lsp::DiagnosticSeverity {
        match self {
            Severity::Error => lsp::DiagnosticSeverity::Error,
            Severity::Warning => lsp::DiagnosticSeverity::Warning,
            Severity::Information => lsp::DiagnosticSeverity::Information,
            Severity::Hint => lsp::DiagnosticSeverity::Hint,
        }
    }
}
