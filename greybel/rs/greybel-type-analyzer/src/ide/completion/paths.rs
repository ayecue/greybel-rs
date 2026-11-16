use greybel_core::{ColumnSettings, Position, Span};
use greybel_documents::Uri;

use super::{CompletionItem, CompletionItemKind};
use crate::analyzer::Analyzer;
use crate::ide::{ImportTarget, RelativePath, TextEdit};

impl Analyzer<'_> {
    pub(super) fn path_completions(
        self,
        uri: &Uri,
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
        let (file, ast, offset) = self.locate(uri, position)?;
        let target = ImportTarget::at(ast.source(), offset as usize, self.dialect())?;
        let db = self.workspace().db();
        let range = ast.lines().range(
            Span::new(
                u32::try_from(target.start).expect("value fits the target integer type"),
                offset,
            ),
            ColumnSettings::default(),
        );
        let mut items: Vec<CompletionItem> = self
            .workspace()
            .files()
            .filter(|&other| other != file && other.text(db).is_some())
            .filter_map(|other| {
                let other = other.uri(db);
                let relative = RelativePath::between(uri, other)?;
                let label = if target.prefix.starts_with('/') {
                    other.path().to_owned()
                } else {
                    relative
                };

                label.starts_with(target.prefix).then(|| CompletionItem {
                    text_edit: Some(TextEdit {
                        range,
                        new_text: label.clone(),
                    }),
                    label,
                    kind: CompletionItemKind::File,
                    detail: other.label(),
                    documentation: None,
                    snippet: None,
                    deprecated: false,
                })
            })
            .collect();

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items.dedup_by(|a, b| a.label == b.label);

        Some(items)
    }
}
