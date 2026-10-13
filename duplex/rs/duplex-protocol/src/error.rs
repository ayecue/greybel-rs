use crate::model::Span;

/// Parse or validation error and source position.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{line}:{column}: {message}", line = span.line, column = span.column)]
pub struct DefinitionError {
    /// Parse or validation failure.
    pub message: String,
    /// Failure source position.
    pub span: Span,
}

impl DefinitionError {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}
