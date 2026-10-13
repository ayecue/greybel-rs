use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DepthDiagnostic {
    #[error("nesting exceeds the limit of {limit} levels")]
    DepthLimit { limit: usize },
}

impl DepthDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::DepthLimit { .. } => ErrorCode::NodeDepthLimit,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::DepthLimit { .. } => {
                Some("Split the expression or block into variables or functions".into())
            }
        }
    }
}
