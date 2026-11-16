use super::{CodeAction, FixContext, QuickFix};
use crate::analyzer::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::Context;

pub(super) struct UnderscoreRename;

impl QuickFix for UnderscoreRename {
    fn applies(&self, kind: DiagnosticKind) -> bool {
        matches!(
            kind,
            DiagnosticKind::UnusedLocal | DiagnosticKind::UnusedParameter
        )
    }

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        Self::action(context, diagnostic).into_iter().collect()
    }
}

impl UnderscoreRename {
    fn action(context: &FixContext<'_>, diagnostic: &Diagnostic) -> Option<CodeAction> {
        let name = Analyzer::text_in(context.ast, diagnostic.range)?;
        let new_name = format!("_{name}");
        let ide_context = context
            .analyzer
            .ide_context(context.uri, diagnostic.range.start)?;

        if ide_context.is_visible(&new_name) {
            return None;
        }

        let edit = context
            .analyzer
            .rename(context.uri, diagnostic.range.start, &new_name)?;

        Some(CodeAction {
            title: format!("Rename to \"{new_name}\""),
            diagnostic: diagnostic.clone(),
            edit,
            is_preferred: false,
        })
    }
}

impl Context<'_> {
    fn is_visible(&self, name: &str) -> bool {
        self.analysis
            .model()
            .resolve_variable(self.scope(), name)
            .is_some()
    }
}
