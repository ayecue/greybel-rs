use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GreyScriptDiagnostic {
    #[error("expected import_code to have opening parenthesis")]
    ImportCodeMissingOpenParen,
    #[error("expected import_code argument to be string literal")]
    ImportCodeArgumentNotString,
    #[error("expected import_code to have closing parenthesis")]
    ImportCodeMissingCloseParen,
}

impl GreyScriptDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        ErrorCode::ExpectedToken
    }
}
