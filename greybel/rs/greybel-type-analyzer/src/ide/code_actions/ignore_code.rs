use greybel_symbols::annotations::{TAG_PREFIX, Tag};

use super::{CodeAction, FixContext, QuickFix};
use crate::analyzer::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::{NameComparison, TextEdit};

pub(super) struct IgnoreCodeSuggestion;

impl QuickFix for IgnoreCodeSuggestion {
    fn applies(&self, kind: DiagnosticKind) -> bool {
        kind == DiagnosticKind::InvalidAnnotation
    }

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let Some(code) = Analyzer::text_in(context.ast, diagnostic.range) else {
            return Vec::new();
        };
        let start = context.offset(diagnostic.range.start).unwrap_or_default() as usize;
        let source = context.ast.source();
        let line_start = source[..start].rfind('\n').map_or(0, |newline| newline + 1);
        let tag = format!("{TAG_PREFIX}{}", Tag::Ignore);

        if !source[line_start..start].contains(&tag) || DiagnosticKind::from_code(code).is_some() {
            return Vec::new();
        }

        NameComparison::new(code)
            .closest(DiagnosticKind::ALL.iter().map(|kind| kind.code()))
            .into_iter()
            .map(|replacement| {
                let edit = TextEdit {
                    range: diagnostic.range,
                    new_text: replacement.to_owned(),
                };

                context.action(
                    diagnostic,
                    format!("Change to \"{replacement}\""),
                    vec![edit],
                )
            })
            .collect()
    }
}
