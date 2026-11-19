use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Position, Span};
use greybel_documents::{SourceFile, Uri};
use greybel_symbols::annotations::{ANY, NEVER, TAG_PREFIX, Tag};
use greybel_symbols::types::{FUNCTION, NULL};

use super::{CompletionItem, CompletionItemKind};
use crate::analyzer::Analyzer;
use crate::api::Api;
use crate::ide::TextEdit;
use crate::ide::text::SourceText;

impl Analyzer<'_> {
    pub(super) fn comment_completions(
        self,
        uri: &Uri,
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let comment = ast
            .comments
            .iter()
            .find(|comment| comment.text_span.start <= offset && offset <= comment.span.end)?;
        let text = &ast.source()[comment.text_span.start as usize..offset as usize];
        let word_start = SourceText::new(text)
            .word_start(text.len(), |c| c.is_alphanumeric() || c == '_' || c == '-');
        let word_span = Span::new(
            comment.text_span.start + u32::try_from(word_start).ok()?,
            offset,
        );
        let edit = |label: &str| TextEdit {
            range: ast.lines().range(word_span, ColumnSettings::default()),
            new_text: label.to_owned(),
        };
        let before_word = &text[..word_start];
        let line_text = before_word.trim_start();

        if line_text.strip_prefix(TAG_PREFIX) == Some("") {
            return Some(
                Tag::ALL
                    .iter()
                    .map(|tag| CompletionItem::doc(tag.as_str(), CompletionItemKind::Keyword, edit))
                    .collect(),
            );
        }

        if let Some(open) = before_word.rfind('{')
            && !before_word[open..].contains('}')
        {
            return Some(
                self.type_names(file)
                    .iter()
                    .map(|name| CompletionItem::doc(name, CompletionItemKind::Type, edit))
                    .collect(),
            );
        }

        let after_param = [Tag::Param, Tag::Params]
            .iter()
            .find_map(|tag| line_text.strip_prefix(&format!("{TAG_PREFIX}{}", tag.as_str())));

        if let Some(after_tag) = after_param {
            let after_type = match after_tag.trim_start().strip_prefix('{') {
                Some(type_and_after) => type_and_after.split_once('}').map(|(_, after)| after),
                None => Some(after_tag),
            };

            if after_type.is_some_and(|after| after.trim().is_empty()) {
                return Some(
                    Self::documented_parameters(ast, comment.span.end)
                        .into_iter()
                        .map(|name| CompletionItem::doc(name, CompletionItemKind::Variable, edit))
                        .collect(),
                );
            }
        }

        Some(Vec::new())
    }

    fn type_names(self, file: SourceFile) -> Vec<String> {
        let api = Api::new(self.dialect());
        let mut names: Vec<String> = [ANY, NULL, NEVER, FUNCTION]
            .into_iter()
            .chain(api.container().visible_types())
            .map(str::to_owned)
            .collect();

        if let Some((analysis, _)) = self.entry_analysis(file) {
            names.extend(analysis.model().custom_types().map(|ty| ty.name.clone()));
        }

        names.sort_unstable();
        names.dedup();

        names
    }

    fn documented_parameters(ast: &Ast, comment_end: u32) -> Vec<&str> {
        let mut next: Option<NodeId> = None;

        Self::first_assignment_after(ast, ast.root(), comment_end, &mut next);

        let Some(statement) = next else {
            return Vec::new();
        };
        let between = ast.slice(Span::new(comment_end, ast.span(statement).start));

        if !between
            .lines()
            .all(|line| line.trim().is_empty() || line.trim_start().starts_with("//"))
        {
            return Vec::new();
        }

        let NodeKind::Assignment(assignment) = ast.kind(statement) else {
            return Vec::new();
        };
        let NodeKind::Function(function) = ast.kind(assignment.init) else {
            return Vec::new();
        };

        ast.list(function.parameters)
            .iter()
            .filter_map(|&parameter| match ast.kind(parameter) {
                NodeKind::Assignment(assignment) => ast.name(assignment.variable),
                _ => ast.name(parameter),
            })
            .collect()
    }

    fn first_assignment_after(ast: &Ast, root: NodeId, offset: u32, next: &mut Option<NodeId>) {
        let mut nodes = ast.descendants(root);

        while let Some(node) = nodes.next() {
            let span = ast.span(node);

            if span.end < offset {
                nodes.skip_children();

                continue;
            }

            if span.start >= offset && matches!(ast.kind(node), NodeKind::Assignment(_)) {
                if next.is_none_or(|current| span.start < ast.span(current).start) {
                    *next = Some(node);
                }

                nodes.skip_children();
            }
        }
    }
}

impl CompletionItem {
    fn doc(label: &str, kind: CompletionItemKind, edit: impl Fn(&str) -> TextEdit) -> Self {
        Self {
            label: label.to_owned(),
            kind,
            detail: String::new(),
            documentation: None,
            text_edit: Some(edit(label)),
            snippet: None,
            deprecated: false,
        }
    }
}
