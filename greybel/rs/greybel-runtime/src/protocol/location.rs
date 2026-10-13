use greybel_core::{ColumnSettings, LineIndex};
use greybel_documents::{AsyncResourceProvider, DocumentError};
use greybel_interpreter::Location;

use super::SourceLocation;

impl SourceLocation {
    /// Rereads a parse error's document and converts its byte span to a source location. Other document errors and
    /// unreadable sources return `None`.
    pub(crate) async fn of_syntax_error(
        error: &DocumentError,
        provider: &impl AsyncResourceProvider,
    ) -> Option<Self> {
        let DocumentError::Parse { target, error } = error else {
            return None;
        };
        let source = provider.read(target).await.ok().flatten()?;
        let position =
            LineIndex::new(&source).position(error.span.start, ColumnSettings::default());

        Some(Self {
            uri: target.to_string(),
            line: position.line,
            character: position.character,
        })
    }
}

impl From<&Location> for SourceLocation {
    fn from(location: &Location) -> Self {
        Self {
            uri: location.uri.to_string(),
            line: location.line,
            character: location.character,
        }
    }
}
