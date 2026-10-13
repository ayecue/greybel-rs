use std::collections::HashMap;

use gen_lsp_types as lsp;
use greybel_type_analyzer::{CodeAction, TextEdit, WorkspaceEdit};

use super::{diagnostic, range, uri};

#[must_use]
pub fn text_edit(edit: &TextEdit) -> lsp::TextEdit {
    lsp::TextEdit {
        range: range(edit.range),
        new_text: edit.new_text.clone(),
    }
}

#[must_use]
pub fn workspace_edit(edit: &WorkspaceEdit) -> lsp::WorkspaceEdit {
    let changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = edit
        .changes
        .iter()
        .map(|(source, edits)| (uri(source), edits.iter().map(text_edit).collect()))
        .collect();

    lsp::WorkspaceEdit {
        changes: Some(changes),
        ..lsp::WorkspaceEdit::default()
    }
}

#[must_use]
pub fn code_action(action: &CodeAction) -> lsp::CodeAction {
    lsp::CodeAction {
        title: action.title.clone(),
        kind: Some(lsp::CodeActionKind::QuickFix),
        diagnostics: Some(vec![diagnostic(&action.diagnostic)]),
        is_preferred: Some(action.is_preferred),
        edit: Some(workspace_edit(&action.edit)),
        ..lsp::CodeAction::default()
    }
}
