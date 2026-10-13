use gen_lsp_types as lsp;
use greybel_type_analyzer::{Diagnostic, Severity};

use super::range;

const SOURCE: &str = "greybel";

#[must_use]
pub fn diagnostic(diagnostic: &Diagnostic) -> lsp::Diagnostic {
    lsp::Diagnostic {
        range: range(diagnostic.range),
        severity: Some(severity(diagnostic.severity)),
        code: Some(lsp::Code::String(diagnostic.kind.code().to_owned())),
        source: Some(SOURCE.to_owned()),
        message: lsp::Message::String(diagnostic.message.clone()),
        ..lsp::Diagnostic::default()
    }
}

fn severity(severity: Severity) -> lsp::DiagnosticSeverity {
    match severity {
        Severity::Error => lsp::DiagnosticSeverity::Error,
        Severity::Warning => lsp::DiagnosticSeverity::Warning,
        Severity::Information => lsp::DiagnosticSeverity::Information,
        Severity::Hint => lsp::DiagnosticSeverity::Hint,
    }
}
