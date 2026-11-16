use greybel_type_analyzer::{DiagnosticKind, Dialect};

use super::diagnostics::diagnostics;
use super::with_document;

pub const KINDS: &[DiagnosticKind] = &[
    DiagnosticKind::UnresolvedReference,
    DiagnosticKind::NativeOverride,
    DiagnosticKind::InvalidAnnotation,
];

/// Kind and line of each reported diagnostic of [`KINDS`] in a MiniScript document.
pub fn reported_lines(source: &str) -> Vec<(DiagnosticKind, u32)> {
    let workspace = with_document(Dialect::MiniScript, source);

    diagnostics(&workspace, KINDS)
        .into_iter()
        .map(|(kind, line, _)| (kind, line))
        .collect()
}
