use greybel_core::{ColumnSettings, Position, Range, Span};
use greybel_documents::Uri;

use super::Analyzer;
use crate::ide::{ColorInformation, Fold, InlayHint, InlayHintCollector, InlayHintOptions};

impl Analyzer<'_> {
    /// The color values of `<color=...>`, `<mark=...>` and `<#...>` tags in string literals.
    #[must_use]
    pub fn document_colors(&self, uri: &Uri) -> Vec<ColorInformation> {
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };

        file.parse(self.workspace.db())
            .ast()
            .map(|ast| ColorInformation::collect(ast))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn folds(&self, uri: &Uri) -> Vec<Fold> {
        let Some(file) = self.workspace.file(uri) else {
            return Vec::new();
        };

        file.parse(self.workspace.db())
            .ast()
            .map(|ast| Fold::collect(ast))
            .unwrap_or_default()
    }

    /// Type and parameter-name hints for the nodes that start within `range`.
    #[must_use]
    pub fn inlay_hints(
        &self,
        uri: &Uri,
        range: Range,
        options: InlayHintOptions,
    ) -> Vec<InlayHint> {
        let Some((file, ast, start)) = self.locate(uri, range.start) else {
            return Vec::new();
        };
        let end = ast
            .lines()
            .offset(range.end, ColumnSettings::default())
            .unwrap_or_else(|| u32::try_from(ast.source().len()).unwrap_or(u32::MAX));
        let Some(&(analysis, document)) = self.analyses_of(file).first() else {
            return Vec::new();
        };

        InlayHintCollector::collect(ast, analysis, document, options, Span::new(start, end))
    }

    /// The ranges of the syntax nodes around `position`, innermost first, each containing the one before.
    #[must_use]
    pub fn selection_ranges(&self, uri: &Uri, position: Position) -> Vec<Range> {
        let Some((_, ast, offset)) = self.locate(uri, position) else {
            return Vec::new();
        };
        let mut ranges: Vec<Range> = Vec::new();

        for node in ast.path_at(offset).into_iter().rev() {
            let span = ast.span(node);

            if span.start == span.end {
                continue;
            }

            let range = ast.lines().range(span, ColumnSettings::default());

            if ranges.last() != Some(&range) {
                ranges.push(range);
            }
        }

        ranges
    }
}
