mod completion;
mod diagnostic;
mod edit;
mod markup;
mod ranges;
mod symbol;

pub use completion::completion_item;
pub use diagnostic::diagnostic;
pub use edit::{code_action, text_edit, workspace_edit};
use gen_lsp_types as lsp;
use greybel_core::{Position, Range};
use greybel_documents::Uri;
use greybel_type_analyzer::Location;
pub use markup::Markup;
pub use ranges::{color_information, color_presentation, fold, inlay_hint, selection_range};
pub use symbol::{call_hierarchy_item, document_symbol, workspace_symbol};

#[must_use]
pub fn uri(uri: &Uri) -> lsp::Uri {
    lsp::Uri(uri.to_string())
}

#[must_use]
pub fn position(position: Position) -> lsp::Position {
    lsp::Position {
        line: position.line.saturating_sub(1),
        character: position.character.saturating_sub(1),
    }
}

#[must_use]
pub fn range(range: Range) -> lsp::Range {
    lsp::Range {
        start: self::position(range.start),
        end: self::position(range.end),
    }
}

#[must_use]
pub fn location(location: &Location) -> lsp::Location {
    lsp::Location {
        uri: uri(&location.uri),
        range: range(location.range),
    }
}
