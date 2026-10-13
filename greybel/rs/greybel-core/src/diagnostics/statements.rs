use crate::error::ErrorCode;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum StatementDiagnostic {
    #[error("sequence expression expected for 'for' loop")]
    ForWithoutSequence,
    #[error("invalid assignment (not an lvalue)")]
    InvalidAssignmentTarget,
}

impl StatementDiagnostic {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::ForWithoutSequence => ErrorCode::ExpectedExpression,
            Self::InvalidAssignmentTarget => ErrorCode::InvalidAssignmentTarget,
        }
    }

    #[must_use]
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::ForWithoutSequence => None,
            Self::InvalidAssignmentTarget => {
                Some("Assign to a variable, a member (a.b) or an element (a[1])".into())
            }
        }
    }
}
