use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GreybelDiagnostic {
    #[error("expected from keyword")]
    MissingFromKeyword,
}

impl GreybelDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::MissingFromKeyword => ErrorCode::ExpectedToken,
        }
    }
}
