use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FunctionDiagnostic {
    #[error("parameter default value must be a literal value")]
    DefaultNotLiteral,
}

impl FunctionDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::DefaultNotLiteral => ErrorCode::InvalidDefaultValue,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::DefaultNotLiteral => {
                Some("Only numbers, strings, booleans and null are allowed as defaults, and in Greybel's own dialect @ references to a name or member".into())
            }
        }
    }
}
