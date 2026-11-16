use greybel_symbols::annotations::{TAG_PREFIX, Tag};

use super::{CodeAction, FixContext, QuickFix};
use crate::database::Annotate;
use crate::diagnostic::{Diagnostic, DiagnosticKind};

pub(super) struct IgnoreComment;

impl QuickFix for IgnoreComment {
    fn applies(&self, kind: DiagnosticKind) -> bool {
        kind != DiagnosticKind::Syntax
    }

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction> {
        let db = context.analyzer.workspace().db();
        let ignores = &context.file.comments(db).ignores;

        if ignores.invalid.contains(diagnostic) {
            return Vec::new();
        }

        let code = diagnostic.kind.code();
        let line = diagnostic.range.start.line;
        let line_edit = match ignores.line_comment(line) {
            Some(comment) => context.comment_extension(comment, code),
            None => {
                let Some(line_start) = context.ast.lines().line_start(line) else {
                    return Vec::new();
                };
                let source = context.ast.source();
                let indent: String = source[line_start as usize..]
                    .chars()
                    .take_while(|&character| character == ' ' || character == '\t')
                    .collect();

                context.insertion(
                    line_start,
                    format!("{indent}// {TAG_PREFIX}{} {code}\n", Tag::Ignore),
                )
            }
        };
        let file_edit = match ignores.file_comment() {
            Some(comment) => context.comment_extension(comment, code),
            None => context.insertion(0, format!("// {TAG_PREFIX}{} {code}\n", Tag::IgnoreFile)),
        };

        vec![
            context.action(
                diagnostic,
                format!("Ignore \"{code}\" on this line"),
                vec![line_edit],
            ),
            context.action(
                diagnostic,
                format!("Ignore \"{code}\" in this file"),
                vec![file_edit],
            ),
        ]
    }
}
