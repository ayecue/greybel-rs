use gen_lsp_types as lsp;
use greybel_type_analyzer::{Diagnostic, DiagnosticKind, Severity};

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
        tags: tag(diagnostic.kind).map(|tag| vec![tag]),
        ..lsp::Diagnostic::default()
    }
}

/// Deprecated uses show struck through; unused and unreachable code shows faded.
fn tag(kind: DiagnosticKind) -> Option<lsp::DiagnosticTag> {
    match kind {
        DiagnosticKind::Deprecated => Some(lsp::DiagnosticTag::Deprecated),
        DiagnosticKind::UnusedLocal
        | DiagnosticKind::UnusedParameter
        | DiagnosticKind::DeadCodeAfterTerminator => Some(lsp::DiagnosticTag::Unnecessary),
        _ => None,
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
