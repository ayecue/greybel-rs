use greybel_core::{Ast, NodeId, NodeKind, Span};

/// Lines are 1-based; `end_line` is the last line that is hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fold {
    pub start_line: u32,
    pub end_line: u32,
    pub kind: FoldKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoldKind {
    Block,
    Comment,
    Imports,
}

impl Fold {
    /// Blocks (function bodies, `if` clauses, loops), multi-line lists and maps, runs of comment lines and runs
    /// of import lines.
    pub(crate) fn collect(ast: &Ast) -> Vec<Self> {
        let mut ranges = Vec::new();

        Self::blocks(ast, ast.root(), &mut ranges);
        Self::comments(ast, &mut ranges);
        Self::imports(ast, &mut ranges);
        ranges.sort_by_key(|range| (range.start_line, range.end_line));
        ranges.dedup();

        ranges
    }

    fn blocks(ast: &Ast, node: NodeId, ranges: &mut Vec<Self>) {
        let line = |offset| ast.lines().line(offset);
        let span = ast.span(node);
        let body = match ast.kind(node) {
            NodeKind::Function(function) => Some(function.body),
            NodeKind::IfClause(clause) if !clause.shortcut => Some(clause.body),
            NodeKind::ElseClause(clause) if !clause.shortcut => Some(clause.body),
            NodeKind::While(statement) => Some(statement.body),
            NodeKind::For(statement) => Some(statement.body),
            _ => None,
        };
        let end_line = match body {
            Some(body) => ast.list(body).last().map(|&last| line(ast.span(last).end)),
            None if matches!(ast.kind(node), NodeKind::Map(_) | NodeKind::List(_)) => {
                Some(line(span.end).saturating_sub(1))
            }
            None => None,
        };

        if let Some(end_line) = end_line {
            Self::push(ranges, line(span.start), end_line, FoldKind::Block);
        }

        ast.for_each_child(node, |child| Self::blocks(ast, child, ranges));
    }

    /// Comments that start their line, on consecutive lines, and block comments over several lines.
    fn comments(ast: &Ast, ranges: &mut Vec<Self>) {
        let mut run: Option<(u32, u32)> = None;

        for comment in &ast.comments {
            let start = ast.lines().line(comment.span.start);
            let end = ast.lines().line(comment.span.end);

            if comment.is_multiline || !Self::starts_line(ast, comment.span) {
                Self::push(ranges, start, end, FoldKind::Comment);
                continue;
            }

            run = match run {
                Some((first, last)) if last + 1 == start => Some((first, start)),
                Some((first, last)) => {
                    Self::push(ranges, first, last, FoldKind::Comment);

                    Some((start, start))
                }
                None => Some((start, start)),
            };
        }

        if let Some((first, last)) = run {
            Self::push(ranges, first, last, FoldKind::Comment);
        }
    }

    /// Import directives on consecutive lines, blank lines between them included.
    fn imports(ast: &Ast, ranges: &mut Vec<Self>) {
        let mut lines: Vec<u32> = [&ast.imports, &ast.includes, &ast.native_imports]
            .into_iter()
            .flatten()
            .map(|&node| ast.lines().line(ast.span(node).start))
            .collect();

        lines.sort_unstable();
        lines.dedup();

        let mut run: Option<(u32, u32)> = None;

        for line in lines {
            run = match run {
                Some((first, last)) if Self::blank_between(ast, last, line) => Some((first, line)),
                Some((first, last)) => {
                    Self::push(ranges, first, last, FoldKind::Imports);

                    Some((line, line))
                }
                None => Some((line, line)),
            };
        }

        if let Some((first, last)) = run {
            Self::push(ranges, first, last, FoldKind::Imports);
        }
    }

    fn starts_line(ast: &Ast, span: Span) -> bool {
        let line = ast.lines().line(span.start);
        let start = ast.lines().line_start(line).unwrap_or(span.start);

        ast.slice(Span::new(start, span.start)).trim().is_empty()
    }

    /// Whether every line strictly between `first` and `last` is blank.
    fn blank_between(ast: &Ast, first: u32, last: u32) -> bool {
        let (Some(start), Some(end)) = (
            ast.lines().line_start(first + 1),
            ast.lines().line_start(last),
        ) else {
            return false;
        };

        start > end || ast.slice(Span::new(start, end)).trim().is_empty()
    }

    fn push(ranges: &mut Vec<Self>, start_line: u32, end_line: u32, kind: FoldKind) {
        if end_line > start_line {
            ranges.push(Self {
                start_line,
                end_line,
                kind,
            });
        }
    }
}
