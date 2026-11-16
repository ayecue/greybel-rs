use std::collections::HashMap;

use gen_lsp_types as lsp;
use greybel_type_analyzer::{CodeAction, TextEdit, WorkspaceEdit};

use super::ToLsp;

impl ToLsp for TextEdit {
    type Output = lsp::TextEdit;

    fn to_lsp(&self) -> lsp::TextEdit {
        lsp::TextEdit {
            range: self.range.to_lsp(),
            new_text: self.new_text.clone(),
        }
    }
}

impl ToLsp for WorkspaceEdit {
    type Output = lsp::WorkspaceEdit;

    fn to_lsp(&self) -> lsp::WorkspaceEdit {
        let changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = self
            .changes
            .iter()
            .map(|(source, edits)| (source.to_lsp(), edits.iter().map(ToLsp::to_lsp).collect()))
            .collect();

        lsp::WorkspaceEdit {
            changes: Some(changes),
            ..lsp::WorkspaceEdit::default()
        }
    }
}

impl ToLsp for CodeAction {
    type Output = lsp::CodeAction;

    fn to_lsp(&self) -> lsp::CodeAction {
        lsp::CodeAction {
            title: self.title.clone(),
            kind: Some(lsp::CodeActionKind::QuickFix),
            diagnostics: Some(vec![self.diagnostic.to_lsp()]),
            is_preferred: Some(self.is_preferred),
            edit: Some(self.edit.to_lsp()),
            ..lsp::CodeAction::default()
        }
    }
}
