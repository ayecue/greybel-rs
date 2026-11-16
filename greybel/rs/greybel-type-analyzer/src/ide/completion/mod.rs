mod comment;
mod keywords;
mod names;
mod paths;

use greybel_core::Position;
use greybel_documents::Uri;

use crate::analyzer::Analyzer;
use crate::ide::TextEdit;
use crate::model::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: String,
    pub documentation: Option<Documentation>,
    pub text_edit: Option<TextEdit>,
    pub snippet: Option<String>,
    pub deprecated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionItemKind {
    Variable,
    Function,
    Property,
    Method,
    File,
    Keyword,
    Snippet,
    Type,
}

impl Analyzer<'_> {
    #[must_use]
    pub fn completions(&self, uri: &Uri, position: Position) -> Vec<CompletionItem> {
        if let Some(paths) = self.path_completions(uri, position) {
            return paths;
        }

        if let Some(items) = self.comment_completions(uri, position) {
            return items;
        }

        self.ide_context(uri, position)
            .map(|context| context.completions())
            .unwrap_or_default()
    }
}
