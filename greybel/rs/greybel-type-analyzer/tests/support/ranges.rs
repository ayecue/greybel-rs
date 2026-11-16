use greybel_core::Position;
use greybel_type_analyzer::{Analyzer, Dialect, FoldKind};

use super::{PATH, uri, with_document};

/// `(start line, end line, kind)` of each folding range of `source`.
pub fn folds(source: &str) -> Vec<(u32, u32, FoldKind)> {
    let workspace = with_document(Dialect::Greybel, source);

    Analyzer::new(&workspace)
        .folds(&uri(PATH))
        .into_iter()
        .map(|range| (range.start_line, range.end_line, range.kind))
        .collect()
}

/// The source text of each selection range at `position`, innermost first.
pub fn selections(source: &str, position: Position) -> Vec<String> {
    let workspace = with_document(Dialect::Greybel, source);
    let lines: Vec<&str> = source.split('\n').collect();

    Analyzer::new(&workspace)
        .selection_ranges(&uri(PATH), position)
        .into_iter()
        .map(|range| {
            let (start, end) = (range.start, range.end);
            let mut text = String::new();

            for line in start.line..=end.line {
                let content = lines[line as usize - 1];
                let from = if line == start.line {
                    start.character as usize - 1
                } else {
                    0
                };
                let to = if line == end.line {
                    end.character as usize - 1
                } else {
                    content.len()
                };

                if line > start.line {
                    text.push('\n');
                }

                text.push_str(&content[from.min(content.len())..to.min(content.len())]);
            }

            text
        })
        .collect()
}

/// `((line, character), (red, green, blue, alpha))`.
pub type ColorAt = ((u32, u32), (u8, u8, u8, u8));

/// Each color value in GreyScript `source`.
pub fn colors(source: &str) -> Vec<ColorAt> {
    let workspace = with_document(Dialect::GreyScript, source);

    Analyzer::new(&workspace)
        .document_colors(&uri(PATH))
        .into_iter()
        .map(|color| {
            let start = color.range.start;

            (
                (start.line, start.character),
                (color.red, color.green, color.blue, color.alpha),
            )
        })
        .collect()
}
