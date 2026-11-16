use gen_lsp_types as lsp;
use greybel_core::{Position, Range};
use greybel_documents::Uri;

pub(crate) trait FromLsp<T> {
    fn from_lsp(value: T) -> Self;
}

pub(crate) trait TryFromLsp<T>: Sized {
    fn try_from_lsp(value: T) -> Option<Self>;
}

impl TryFromLsp<&lsp::Uri> for Uri {
    fn try_from_lsp(uri: &lsp::Uri) -> Option<Self> {
        Uri::parse(&uri.0).ok()
    }
}

impl FromLsp<lsp::Position> for Position {
    fn from_lsp(position: lsp::Position) -> Self {
        Position::new(position.line + 1, position.character + 1)
    }
}

impl FromLsp<lsp::Range> for Range {
    fn from_lsp(range: lsp::Range) -> Self {
        Range::new(
            Position::from_lsp(range.start),
            Position::from_lsp(range.end),
        )
    }
}
