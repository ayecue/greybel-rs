use greybel_core::{ColumnSettings, LineIndex};
use greybel_documents::{AsyncResourceProvider, DocumentError};

use super::SourceLocation;

impl SourceLocation {
    /// Reads the target of a parse error and converts its span to a line and character. Returns `None` for other
    /// errors or unreadable sources.
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
