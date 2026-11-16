use gen_lsp_types as lsp;
use greybel_type_analyzer::{CompletionItem, CompletionItemKind};

use super::ToLsp;

impl ToLsp for CompletionItem {
    type Output = lsp::CompletionItem;

    fn to_lsp(&self) -> lsp::CompletionItem {
        lsp::CompletionItem {
            label: self.label.clone(),
            kind: Some(self.kind.to_lsp()),
            detail: (!self.detail.is_empty()).then(|| self.detail.clone()),
            text_edit: self
                .text_edit
                .as_ref()
                .map(|edit| lsp::CompletionItemTextEdit::TextEdit(edit.to_lsp())),
            insert_text: self.snippet.clone(),
            insert_text_format: self
                .snippet
                .is_some()
                .then_some(lsp::InsertTextFormat::Snippet),
            tags: self
                .deprecated
                .then(|| vec![lsp::CompletionItemTag::Deprecated]),
            ..lsp::CompletionItem::default()
        }
    }
}

impl ToLsp for CompletionItemKind {
    type Output = lsp::CompletionItemKind;

    fn to_lsp(&self) -> lsp::CompletionItemKind {
        match self {
            CompletionItemKind::Variable => lsp::CompletionItemKind::Variable,
            CompletionItemKind::Function => lsp::CompletionItemKind::Function,
            CompletionItemKind::Property => lsp::CompletionItemKind::Property,
            CompletionItemKind::Method => lsp::CompletionItemKind::Method,
            CompletionItemKind::File => lsp::CompletionItemKind::File,
            CompletionItemKind::Keyword => lsp::CompletionItemKind::Keyword,
            CompletionItemKind::Snippet => lsp::CompletionItemKind::Snippet,
            CompletionItemKind::Type => lsp::CompletionItemKind::Class,
        }
    }
}
