mod completion;
mod diagnostic;
mod edit;
mod markup;
mod navigation;
mod ranges;
mod symbol;
mod tokens;

use gen_lsp_types as lsp;
use greybel_core::{Position, Range};
use greybel_documents::Uri;
use greybel_type_analyzer::Location;
pub(crate) use markup::Markup;
pub(crate) use navigation::{DefinitionLink, DocumentHighlight};
pub(crate) use ranges::{ColorPresentation, SelectionRange};
pub(crate) use symbol::{IncomingCall, OutgoingCall};
pub(crate) use tokens::semantic_tokens;

pub(crate) trait ToLsp {
    type Output;

    fn to_lsp(&self) -> Self::Output;
}

impl ToLsp for Uri {
    type Output = lsp::Uri;

    fn to_lsp(&self) -> lsp::Uri {
        lsp::Uri(self.to_string())
    }
}

impl ToLsp for Position {
    type Output = lsp::Position;

    fn to_lsp(&self) -> lsp::Position {
        lsp::Position {
            line: self.line.saturating_sub(1),
            character: self.character.saturating_sub(1),
        }
    }
}

impl ToLsp for Range {
    type Output = lsp::Range;

    fn to_lsp(&self) -> lsp::Range {
        lsp::Range {
            start: self.start.to_lsp(),
            end: self.end.to_lsp(),
        }
    }
}

impl ToLsp for Location {
    type Output = lsp::Location;

    fn to_lsp(&self) -> lsp::Location {
        lsp::Location {
            uri: self.uri.to_lsp(),
            range: self.range.to_lsp(),
        }
    }
}
