use greybel_core::{Ast, Span};
use greybel_symbols::annotations::{TAG_PREFIX, Tag};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::diagnostic::{Diagnostic, DiagnosticKind};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IgnoreTable {
    file: Option<DiagnosticSelection>,
    lines: FxHashMap<u32, DiagnosticSelection>,
    /// The last `@ignore-file` and the last `@ignore` per line that name codes, which a quick fix extends.
    file_comment: Option<Span>,
    line_comments: FxHashMap<u32, Span>,
    pub invalid: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiagnosticSelection {
    All,
    Kinds(FxHashSet<DiagnosticKind>),
}

impl DiagnosticSelection {
    fn contains(&self, kind: DiagnosticKind) -> bool {
        match self {
            Self::All => true,
            Self::Kinds(kinds) => kinds.contains(&kind),
        }
    }

    fn extend(&mut self, other: Self) {
        match (&mut *self, other) {
            (Self::All, _) => {}
            (_, Self::All) => *self = Self::All,
            (Self::Kinds(kinds), Self::Kinds(other)) => kinds.extend(other),
        }
    }
}

impl IgnoreTable {
    pub fn new(ast: &Ast, code_lines: &[u32]) -> Self {
        let lines = ast.lines();
        let mut table = Self::default();

        for comment in &ast.comments {
            if comment.is_multiline {
                continue;
            }

            let Some((tag, codes)) = table.read(ast, comment.text_span) else {
                continue;
            };
            let line = lines.line(comment.span.start);
            let names_codes = matches!(codes, DiagnosticSelection::Kinds(_));

            match tag {
                Tag::IgnoreFile => {
                    if names_codes {
                        table.file_comment = Some(comment.text_span);
                    }

                    match &mut table.file {
                        Some(file) => file.extend(codes),
                        None => table.file = Some(codes),
                    }
                }
                _ => {
                    let own_line = code_lines.binary_search(&line).is_err();
                    let target = if own_line {
                        let next = code_lines.partition_point(|&code_line| code_line <= line);

                        code_lines.get(next).copied()
                    } else {
                        Some(line)
                    };

                    if let Some(target) = target {
                        if names_codes {
                            table.line_comments.insert(target, comment.text_span);
                        }

                        match table.lines.get_mut(&target) {
                            Some(selection) => selection.extend(codes),
                            None => {
                                table.lines.insert(target, codes);
                            }
                        }
                    }
                }
            }
        }

        table
    }

    #[must_use]
    pub fn ignores(&self, diagnostic: &Diagnostic) -> bool {
        let selects = |selection: &DiagnosticSelection| selection.contains(diagnostic.kind);

        self.file.as_ref().is_some_and(selects)
            || self
                .lines
                .get(&diagnostic.range.start.line)
                .is_some_and(selects)
    }

    pub fn file_comment(&self) -> Option<Span> {
        self.file_comment
    }

    pub fn line_comment(&self, line: u32) -> Option<Span> {
        self.line_comments.get(&line).copied()
    }

    fn read(&mut self, ast: &Ast, text_span: Span) -> Option<(Tag, DiagnosticSelection)> {
        let text = ast.slice(text_span).trim_start();
        let tag_text = text.strip_prefix(TAG_PREFIX)?;
        let (tag, remainder) = tag_text
            .split_once(char::is_whitespace)
            .unwrap_or((tag_text, ""));
        let tag = Tag::parse(tag).filter(|tag| matches!(tag, Tag::Ignore | Tag::IgnoreFile))?;
        let mut kinds = FxHashSet::default();
        let mut named = false;

        for code in remainder
            .split(|character: char| character == ',' || character.is_whitespace())
            .filter(|code| !code.is_empty())
        {
            named = true;

            match DiagnosticKind::from_code(code) {
                Some(kind) => {
                    kinds.insert(kind);
                }
                None => {
                    let comment = ast.slice(text_span);
                    let start = text_span.start
                        + u32::try_from(code.as_ptr().addr() - comment.as_ptr().addr())
                            .unwrap_or(0);
                    let end = start + u32::try_from(code.len()).unwrap_or(0);

                    self.invalid.push(Diagnostic::new(
                        DiagnosticKind::InvalidAnnotation,
                        ast.lines(),
                        Span::new(start, end),
                        format!("@{tag} names no diagnostic \"{code}\"."),
                    ));
                }
            }
        }

        let codes = if named {
            DiagnosticSelection::Kinds(kinds)
        } else {
            DiagnosticSelection::All
        };

        Some((tag, codes))
    }
}
