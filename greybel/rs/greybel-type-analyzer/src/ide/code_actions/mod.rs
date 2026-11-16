mod ignore_code;
mod ignore_comment;
mod include;
mod name_suggestion;
mod underscore;
mod unreachable_removal;
mod unused_removal;

use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Position, Range, Span, UnaryOp};
use greybel_documents::{SourceFile, Uri};
use ignore_code::IgnoreCodeSuggestion;
use ignore_comment::IgnoreComment;
use include::AddInclude;
use name_suggestion::NameSuggestion;
use underscore::UnderscoreRename;
use unreachable_removal::UnreachableRemoval;
use unused_removal::UnusedAssignmentRemoval;

use crate::analyzer::Analyzer;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::{TextEdit, WorkspaceEdit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub diagnostic: Diagnostic,
    pub edit: WorkspaceEdit,
    pub is_preferred: bool,
}

trait QuickFix {
    fn applies(&self, kind: DiagnosticKind) -> bool;

    fn actions(&self, context: &FixContext<'_>, diagnostic: &Diagnostic) -> Vec<CodeAction>;
}

const DIAGNOSTIC_FIXES: [&dyn QuickFix; 5] = [
    &NameSuggestion,
    &UnusedAssignmentRemoval,
    &UnderscoreRename,
    &IgnoreCodeSuggestion,
    &UnreachableRemoval,
];

struct FixContext<'a> {
    analyzer: Analyzer<'a>,
    uri: &'a Uri,
    file: SourceFile,
    ast: &'a Ast,
}

impl FixContext<'_> {
    fn action(&self, diagnostic: &Diagnostic, title: String, edits: Vec<TextEdit>) -> CodeAction {
        CodeAction {
            title,
            diagnostic: diagnostic.clone(),
            edit: WorkspaceEdit::of_file(self.uri, edits),
            is_preferred: false,
        }
    }

    fn range(&self, span: Span) -> Range {
        self.ast.lines().range(span, ColumnSettings::default())
    }

    fn offset(&self, position: Position) -> Option<u32> {
        self.ast.lines().offset(position, ColumnSettings::default())
    }

    fn insertion(&self, offset: u32, new_text: String) -> TextEdit {
        TextEdit {
            range: self.range(Span::empty(offset)),
            new_text,
        }
    }

    fn body(ast: &Ast, node: NodeId) -> Option<&[NodeId]> {
        let body = match ast.kind(node) {
            NodeKind::Chunk(chunk) => chunk.body,
            NodeKind::Function(function) => function.body,
            NodeKind::IfClause(clause) => clause.body,
            NodeKind::ElseClause(clause) => clause.body,
            NodeKind::While(statement) => statement.body,
            NodeKind::For(statement) => statement.body,
            _ => return None,
        };

        Some(ast.list(body))
    }

    fn comment_extension(&self, comment: Span, code: &str) -> TextEdit {
        let text = self.ast.slice(comment).trim_end();
        let end = comment.start + u32::try_from(text.len()).expect("comment fits u32");

        self.insertion(end, format!(", {code}"))
    }

    fn has_side_effects(&self, node: NodeId) -> bool {
        let ast = self.ast;

        match ast.kind(node) {
            NodeKind::Literal(_) | NodeKind::Function(_) => false,
            NodeKind::Unary(unary) => match unary.operator {
                UnaryOp::AddressOf => !matches!(ast.kind(unary.argument), NodeKind::Identifier(_)),
                UnaryOp::Negate => !matches!(ast.kind(unary.argument), NodeKind::Literal(_)),
                UnaryOp::Not | UnaryOp::New => true,
            },
            NodeKind::Parenthesis(parenthesis) => self.has_side_effects(parenthesis.expression),
            NodeKind::List(list) => ast
                .list(list.fields)
                .iter()
                .any(|&item| self.has_side_effects(item)),
            NodeKind::ListItem(item) => self.has_side_effects(item.value),
            NodeKind::Map(map) => ast
                .list(map.fields)
                .iter()
                .any(|&entry| self.has_side_effects(entry)),
            NodeKind::MapEntry(entry) => {
                self.has_side_effects(entry.key) || self.has_side_effects(entry.value)
            }
            _ => true,
        }
    }

    fn statement_lines(&self, span: Span) -> Option<Span> {
        let source = self.ast.source();
        let start = span.start as usize;
        let end = span.end as usize;
        let line_start = source[..start].rfind('\n').map_or(0, |newline| newline + 1);
        let line_end = source[end..]
            .find('\n')
            .map_or(source.len(), |newline| end + newline + 1);
        let fills_lines =
            source[line_start..start].trim().is_empty() && source[end..line_end].trim().is_empty();

        fills_lines.then(|| {
            Span::new(
                u32::try_from(line_start).expect("offset fits u32"),
                u32::try_from(line_end).expect("offset fits u32"),
            )
        })
    }
}

impl Analyzer<'_> {
    #[must_use]
    pub fn code_actions(&self, uri: &Uri, range: Range) -> Vec<CodeAction> {
        let Some(file) = self.workspace().file(uri) else {
            return Vec::new();
        };
        let Some(ast) = file.parse(self.workspace().db()).ast() else {
            return Vec::new();
        };
        let diagnostics: Vec<Diagnostic> = self
            .diagnostics(uri)
            .into_iter()
            .filter(|diagnostic| {
                diagnostic.range.start <= range.end && range.start <= diagnostic.range.end
            })
            .collect();
        let context = FixContext {
            analyzer: *self,
            uri,
            file,
            ast,
        };
        let mut actions = AddInclude::actions(&context, &diagnostics);

        for diagnostic in &diagnostics {
            for fix in DIAGNOSTIC_FIXES {
                if fix.applies(diagnostic.kind) {
                    actions.extend(fix.actions(&context, diagnostic));
                }
            }
        }

        for diagnostic in &diagnostics {
            if IgnoreComment.applies(diagnostic.kind) {
                actions.extend(IgnoreComment.actions(&context, diagnostic));
            }
        }

        actions
    }
}
