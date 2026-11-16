//! Helpers for the tests of `self` in methods of maps with instances.

use greybel_type_analyzer::{Analyzer, Dialect};

use crate::support::{PATH, marked, uri, with_document};

/// Hover texts at the `|` markers of MiniScript `source`.
pub fn hovers(source: &str) -> Vec<Option<String>> {
    let (source, positions) = marked(source);
    let workspace = with_document(Dialect::MiniScript, &source);

    positions
        .into_iter()
        .map(|position| {
            Analyzer::new(&workspace)
                .hover(&uri(PATH), position)
                .map(|hover| hover.text)
        })
        .collect()
}
