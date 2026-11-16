use greybel_core::Span;

use super::{CodeAction, FixContext, QuickFix};
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::TextEdit;

pub(super) struct UnreachableRemoval;

impl QuickFix for UnreachableRemoval {
    fn applies(&self, kind: DiagnosticKind) -> bool {
        kind == DiagnosticKind::DeadCodeAfterTerminator
    }

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        Self::action(context, diagnostic).into_iter().collect()
    }
}

impl UnreachableRemoval {
    fn action(context: &FixContext<'_>, diagnostic: &Diagnostic) -> Option<CodeAction> {
        let ast = context.ast;
        let start = context.offset(diagnostic.range.start)?;
        let path = ast.path_at(start);
        let (body, index) = path.windows(2).find_map(|pair| {
            let body = FixContext::body(ast, pair[0])?;
            let index = body.iter().position(|&statement| statement == pair[1])?;

            (ast.span(pair[1]).start == start).then_some((body, index))
        })?;
        let last = *body.last()?;
        let lines = context.statement_lines(Span::new(start, ast.span(last).end))?;
        let edit = TextEdit {
            range: context.range(lines),
            new_text: String::new(),
        };
        let title = if index + 1 == body.len() {
            "Remove the unreachable statement"
        } else {
            "Remove the unreachable statements"
        };

        Some(context.action(diagnostic, title.to_owned(), vec![edit]))
    }
}
