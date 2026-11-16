use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GreyScriptDiagnostic {
    #[error("expected '(' after import_code")]
    ImportCodeMissingOpenParen,
    #[error("expected a string literal as the import_code argument")]
    ImportCodeArgumentNotString,
    #[error("expected ')' after the import_code argument")]
    ImportCodeMissingCloseParen,
}

impl GreyScriptDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        ErrorCode::ExpectedToken
    }
}
