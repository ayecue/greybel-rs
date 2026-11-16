use greybel_type_analyzer::{Analyzer, CompletionItemKind, Dialect};

use super::{PATH, marked, uri, with_document};

/// The labels of the keyword and snippet completions at the `|` in `source`.
pub fn keyword_labels(source: &str) -> Vec<String> {
    let (source, positions) = marked(source);
    let workspace = with_document(Dialect::Greybel, &source);

    Analyzer::new(&workspace)
        .completions(&uri(PATH), positions[0])
        .into_iter()
        .filter(|item| {
            matches!(
                item.kind,
                CompletionItemKind::Keyword | CompletionItemKind::Snippet
            )
        })
        .map(|item| item.label)
        .collect()
}

/// The snippet text of the completion `label` at the `|` in `source`.
pub fn snippet(source: &str, label: &str) -> Option<String> {
    let (source, positions) = marked(source);
    let workspace = with_document(Dialect::Greybel, &source);

    Analyzer::new(&workspace)
        .completions(&uri(PATH), positions[0])
        .into_iter()
        .find(|item| item.label == label && item.kind == CompletionItemKind::Snippet)?
        .snippet
}

/// The labels of every completion at the `|` in GreyScript `source`.
pub fn labels_at(source: &str) -> Vec<String> {
    let (source, positions) = marked(source);
    let workspace = with_document(Dialect::GreyScript, &source);

    Analyzer::new(&workspace)
        .completions(&uri(PATH), positions[0])
        .into_iter()
        .map(|item| item.label)
        .collect()
}
