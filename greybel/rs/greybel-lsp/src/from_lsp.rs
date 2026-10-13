use gen_lsp_types as lsp;
use greybel_core::{Position, Range};
use greybel_documents::Uri;

#[must_use]
pub fn uri(uri: &lsp::Uri) -> Option<Uri> {
    Uri::parse(&uri.0).ok()
}

#[must_use]
pub fn position(position: lsp::Position) -> Position {
    Position::new(position.line + 1, position.character + 1)
}

#[must_use]
pub fn range(range: lsp::Range) -> Range {
    Range::new(position(range.start), position(range.end))
}
