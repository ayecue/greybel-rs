use greybel_core::{ColumnSettings, LineIndex, Position, Range};
use greybel_type_analyzer::{Analyzer, Dialect, TextEdit};

use super::{PATH, uri, with_document};

/// Each code action on `line` of a Greybel document as its title and the source after applying its edits.
pub fn fixes(source: &str, line: u32) -> Vec<(String, String)> {
    let workspace = with_document(Dialect::Greybel, source);
    let range = Range::new(Position::new(line, 1), Position::new(line, 200));

    Analyzer::new(&workspace)
        .code_actions(&uri(PATH), range)
        .into_iter()
        .map(|action| {
            let edits = action
                .edit
                .changes
                .get(&uri(PATH))
                .cloned()
                .unwrap_or_default();

            (action.title, applied(source, &edits))
        })
        .collect()
}

/// The titles of [`fixes`].
pub fn titles(source: &str, line: u32) -> Vec<String> {
    fixes(source, line)
        .into_iter()
        .map(|(title, _)| title)
        .collect()
}

/// The source after the fix titled `title`.
pub fn fixed(source: &str, line: u32, title: &str) -> Option<String> {
    fixes(source, line)
        .into_iter()
        .find_map(|(fix_title, result)| (fix_title == title).then_some(result))
}

fn applied(source: &str, edits: &[TextEdit]) -> String {
    let lines = LineIndex::new(source);
    let mut spans: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|edit| {
            let offset = |position| {
                lines
                    .offset(position, ColumnSettings::default())
                    .expect("edit inside the source") as usize
            };

            (
                offset(edit.range.start),
                offset(edit.range.end),
                edit.new_text.as_str(),
            )
        })
        .collect();
    let mut result = source.to_owned();

    spans.sort_by_key(|&(start, _, _)| std::cmp::Reverse(start));

    for (start, end, text) in spans {
        result.replace_range(start..end, text);
    }

    result
}
