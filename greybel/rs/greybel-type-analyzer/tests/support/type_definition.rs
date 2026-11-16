use greybel_core::Position;
use greybel_type_analyzer::{Analyzer, Dialect};

use super::{PATH, uri, with_document};

/// `(start line, start character, end line, end character)` of each type definition at `position` of `source`.
pub fn type_definitions(source: &str, position: Position) -> Vec<(u32, u32, u32, u32)> {
    let workspace = with_document(Dialect::Greybel, source);

    Analyzer::new(&workspace)
        .type_definition(&uri(PATH), position)
        .into_iter()
        .map(|location| {
            let range = location.range;

            (
                range.start.line,
                range.start.character,
                range.end.line,
                range.end.character,
            )
        })
        .collect()
}
