use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Position, Range, Span, UnaryOp};
use greybel_documents::{SourceFile, Uri};
use greybel_symbols::Keyword;
use greybel_symbols::annotations::{TAG_PREFIX, Tag};

use super::Analyzer;
use crate::database::Annotate;
use crate::diagnostic::{Diagnostic, DiagnosticKind};
use crate::ide::{CodeAction, NameComparison, TextEdit, WorkspaceEdit};

struct Fix<'a> {
    uri: &'a Uri,
    file: SourceFile,
    ast: &'a Ast,
    diagnostic: &'a Diagnostic,
}

impl Fix<'_> {
    fn action(&self, title: String, edits: Vec<TextEdit>) -> CodeAction {
        CodeAction {
            title,
            diagnostic: self.diagnostic.clone(),
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

    /// "Change to ..." for a code in an `@ignore` comment that names no diagnostic, with the closest codes.
    fn ignore_code_actions(&self) -> Vec<CodeAction> {
        let Some(code) = Analyzer::text_in(self.ast, self.diagnostic.range) else {
            return Vec::new();
        };
        let start = self.offset(self.diagnostic.range.start).unwrap_or_default() as usize;
        let source = self.ast.source();
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
                    range: self.diagnostic.range,
                    new_text: replacement.to_owned(),
                };

                self.action(format!("Change to \"{replacement}\""), vec![edit])
            })
            .collect()
    }

    /// Removes an assignment to an unused local when it is on lines of its own and its value has no side effects.
    fn removal_action(&self) -> Option<CodeAction> {
        let ast = self.ast;
        let start = self.offset(self.diagnostic.range.start)?;
        let end = self.offset(self.diagnostic.range.end)?;
        let assignment = ast.path_at(start).into_iter().rev().find_map(|node| {
            let NodeKind::Assignment(assignment) = ast.kind(node) else {
                return None;
            };
            let variable = ast.span(assignment.variable);

            (variable.start == start && variable.end == end && !assignment.shorthand)
                .then_some((node, assignment.init))
        });
        let (statement, value) = assignment?;

        if self.has_side_effects(value) {
            return None;
        }

        let lines = self.statement_lines(ast.span(statement))?;
        let name = Analyzer::text_in(ast, self.diagnostic.range)?;
        let edit = TextEdit {
            range: self.range(lines),
            new_text: String::new(),
        };

        Some(self.action(format!("Remove the assignment to \"{name}\""), vec![edit]))
    }

    /// Removes the unreachable statement and the statements after it in its block, when they fill their lines.
    fn unreachable_removal_action(&self) -> Option<CodeAction> {
        let ast = self.ast;
        let start = self.offset(self.diagnostic.range.start)?;
        let path = ast.path_at(start);
        let (body, index) = path.windows(2).find_map(|pair| {
            let body = Self::body(ast, pair[0])?;
            let index = body.iter().position(|&statement| statement == pair[1])?;

            (ast.span(pair[1]).start == start).then_some((body, index))
        })?;
        let last = *body.last()?;
        let lines = self.statement_lines(Span::new(start, ast.span(last).end))?;
        let edit = TextEdit {
            range: self.range(lines),
            new_text: String::new(),
        };
        let title = if index + 1 == body.len() {
            "Remove the unreachable statement"
        } else {
            "Remove the unreachable statements"
        };

        Some(self.action(title.to_owned(), vec![edit]))
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

    /// Appends `, code` to an `@ignore` or `@ignore-file` comment.
    fn comment_extension(&self, comment: Span, code: &str) -> TextEdit {
        let text = self.ast.slice(comment).trim_end();
        let end = comment.start + u32::try_from(text.len()).expect("comment fits u32");

        self.insertion(end, format!(", {code}"))
    }

    /// Whether evaluating `node` can have side effects: all but literals, `@name`, function literals and lists or
    /// maps of those.
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

    /// The complete lines of a statement with their line break, when nothing else is on them.
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
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };
        let Some(ast) = file.parse(self.workspace.db()).ast() else {
            return Vec::new();
        };
        let diagnostics: Vec<Diagnostic> = self
            .diagnostics(uri)
            .into_iter()
            .filter(|diagnostic| {
                diagnostic.range.start <= range.end && range.start <= diagnostic.range.end
            })
            .collect();
        let missing_names: Vec<&Diagnostic> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == DiagnosticKind::UnresolvedReference)
            .collect();
        let mut actions = self.include_actions(uri, file, ast, &missing_names);

        for diagnostic in &diagnostics {
            let fix = Fix {
                uri,
                file,
                ast,
                diagnostic,
            };

            match diagnostic.kind {
                DiagnosticKind::UnresolvedReference | DiagnosticKind::UnknownMember => {
                    actions.extend(self.name_actions(&fix));
                }
                DiagnosticKind::UnusedLocal => {
                    actions.extend(fix.removal_action());
                    actions.extend(self.underscore_action(&fix));
                }
                DiagnosticKind::UnusedParameter => actions.extend(self.underscore_action(&fix)),
                DiagnosticKind::InvalidAnnotation => actions.extend(fix.ignore_code_actions()),
                DiagnosticKind::DeadCodeAfterTerminator => {
                    actions.extend(fix.unreachable_removal_action());
                }
                _ => {}
            }
        }

        for diagnostic in &diagnostics {
            let fix = Fix {
                uri,
                file,
                ast,
                diagnostic,
            };

            actions.extend(self.ignore_actions(&fix));
        }

        actions
    }

    /// Replacements for a name that isn't found: `self.name` in a method whose `self` has that member, then close
    /// names in scope (or close members after a `.`), and `return`, `break` or `continue` at the start of a line.
    fn name_actions(self, fix: &Fix<'_>) -> Vec<CodeAction> {
        let Some(name) = Self::text_in(fix.ast, fix.diagnostic.range) else {
            return Vec::new();
        };
        let Some(context) = self.ide_context(fix.uri, fix.diagnostic.range.end) else {
            return Vec::new();
        };
        let mut replacements: Vec<String> = Vec::new();

        if fix.diagnostic.kind == DiagnosticKind::UnresolvedReference {
            replacements.extend(
                context
                    .receiver_members(name)
                    .into_iter()
                    .map(|member| format!("self.{member}")),
            );
        }

        let source = fix.ast.source();
        let start = fix.offset(fix.diagnostic.range.start).unwrap_or_default() as usize;
        let line_start = source[..start].rfind('\n').map_or(0, |newline| newline + 1);
        let starts_statement = fix.diagnostic.kind == DiagnosticKind::UnresolvedReference
            && source[line_start..start].trim().is_empty();
        let keywords: &[&str] = if starts_statement {
            &[
                Keyword::Return.as_str(),
                Keyword::Break.as_str(),
                Keyword::Continue.as_str(),
            ]
        } else {
            &[]
        };

        replacements.extend(context.closest_completions(name, keywords));

        replacements
            .into_iter()
            .map(|replacement| {
                let edit = TextEdit {
                    range: fix.diagnostic.range,
                    new_text: replacement.clone(),
                };

                fix.action(format!("Change to \"{replacement}\""), vec![edit])
            })
            .collect()
    }

    /// Renames an unused variable to `_name`, which the unused checks skip.
    fn underscore_action(self, fix: &Fix<'_>) -> Option<CodeAction> {
        let name = Self::text_in(fix.ast, fix.diagnostic.range)?;
        let new_name = format!("_{name}");
        let context = self.ide_context(fix.uri, fix.diagnostic.range.start)?;

        if context.is_visible(&new_name) {
            return None;
        }

        let edit = self.rename(fix.uri, fix.diagnostic.range.start, &new_name)?;

        Some(CodeAction {
            title: format!("Rename to \"{new_name}\""),
            diagnostic: fix.diagnostic.clone(),
            edit,
            is_preferred: false,
        })
    }

    /// `// @ignore <code>` for the diagnostic's line and `// @ignore-file <code>`, each extending an existing comment
    /// that names codes.
    fn ignore_actions(self, fix: &Fix<'_>) -> Vec<CodeAction> {
        let db = self.workspace.db();
        let ignores = &fix.file.comments(db).ignores;
        let diagnostic = fix.diagnostic;

        if diagnostic.kind == DiagnosticKind::Syntax || ignores.invalid.contains(diagnostic) {
            return Vec::new();
        }

        let code = diagnostic.kind.code();
        let line = diagnostic.range.start.line;
        let line_edit = match ignores.line_comment(line) {
            Some(comment) => fix.comment_extension(comment, code),
            None => {
                let Some(line_start) = fix.ast.lines().line_start(line) else {
                    return Vec::new();
                };
                let source = fix.ast.source();
                let indent: String = source[line_start as usize..]
                    .chars()
                    .take_while(|&character| character == ' ' || character == '\t')
                    .collect();

                fix.insertion(
                    line_start,
                    format!("{indent}// {TAG_PREFIX}{} {code}\n", Tag::Ignore),
                )
            }
        };
        let file_edit = match ignores.file_comment() {
            Some(comment) => fix.comment_extension(comment, code),
            None => fix.insertion(0, format!("// {TAG_PREFIX}{} {code}\n", Tag::IgnoreFile)),
        };

        vec![
            fix.action(format!("Ignore \"{code}\" on this line"), vec![line_edit]),
            fix.action(format!("Ignore \"{code}\" in this file"), vec![file_edit]),
        ]
    }
}
