use gen_lsp_types as lsp;
use greybel_type_analyzer::{CompletionItem, CompletionItemKind};

use super::text_edit;

#[must_use]
pub fn completion_item(
    item: &CompletionItem,
    data: Option<serde_json::Value>,
) -> lsp::CompletionItem {
    lsp::CompletionItem {
        label: item.label.clone(),
        kind: Some(completion_kind(item.kind)),
        detail: (!item.detail.is_empty()).then(|| item.detail.clone()),
        text_edit: item
            .text_edit
            .as_ref()
            .map(|edit| lsp::CompletionItemTextEdit::TextEdit(text_edit(edit))),
        data,
        ..lsp::CompletionItem::default()
    }
}

fn completion_kind(kind: CompletionItemKind) -> lsp::CompletionItemKind {
    match kind {
        CompletionItemKind::Variable => lsp::CompletionItemKind::Variable,
        CompletionItemKind::Function => lsp::CompletionItemKind::Function,
        CompletionItemKind::Property => lsp::CompletionItemKind::Property,
        CompletionItemKind::Method => lsp::CompletionItemKind::Method,
        CompletionItemKind::File => lsp::CompletionItemKind::File,
    }
}
