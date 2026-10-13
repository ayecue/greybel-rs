use super::WorkspaceEdit;
use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub diagnostic: Diagnostic,
    pub edit: WorkspaceEdit,
    pub is_preferred: bool,
}
