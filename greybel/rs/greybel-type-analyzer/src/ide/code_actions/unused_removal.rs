use greybel_core::NodeKind;

use super::{CodeAction, FixContext, QuickFix};
use crate::analyzer::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::TextEdit;

pub(super) struct UnusedAssignmentRemoval;

impl QuickFix for UnusedAssignmentRemoval {
    fn applies(&self, kind: DiagnosticKind) -> bool {
        kind == DiagnosticKind::UnusedLocal
    }

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        Self::action(context, diagnostic).into_iter().collect()
    }
}

impl UnusedAssignmentRemoval {
    fn action(context: &FixContext<'_>, diagnostic: &Diagnostic) -> Option<CodeAction> {
        let ast = context.ast;
        let start = context.offset(diagnostic.range.start)?;
        let end = context.offset(diagnostic.range.end)?;
        let assignment = ast.path_at(start).into_iter().rev().find_map(|node| {
            let NodeKind::Assignment(assignment) = ast.kind(node) else {
                return None;
            };
            let variable = ast.span(assignment.variable);

            (variable.start == start && variable.end == end && !assignment.shorthand)
                .then_some((node, assignment.init))
        });
        let (statement, value) = assignment?;

        if context.has_side_effects(value) {
            return None;
        }

        let lines = context.statement_lines(ast.span(statement))?;
        let name = Analyzer::text_in(ast, diagnostic.range)?;
        let edit = TextEdit {
            range: context.range(lines),
            new_text: String::new(),
        };

        Some(context.action(
            diagnostic,
            format!("Remove the assignment to \"{name}\""),
            vec![edit],
        ))
    }
}
