mod node;
mod type_name;

use greybel_core::{Position, Range};
use greybel_documents::Uri;

use crate::analyzer::Analyzer;
use crate::model::Documentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub range: Range,
    pub text: String,
    pub kind: HoverKind,
    pub documentation: Option<Documentation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverKind {
    #[default]
    Signature,
    Sentence,
}

impl Analyzer<'_> {
    #[must_use]
    pub fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        if let Some(hover) = self.import_hover(uri, position) {
            return Some(hover);
        }

        if let Some(hover) = self.type_hover(uri, position) {
            return Some(hover);
        }

        self.ide_context(uri, position)?.hover()
    }
}
