use crate::definition::{ErrorCode, Violation};
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Rejection {
    Code(ErrorCode),
    Invalid(Box<Violation>),
}

impl Rejection {
    pub(crate) fn of(error: &Error) -> Self {
        match error {
            Error::Unavailable => Self::Code(ErrorCode::UnsupportedVersion),
            Error::Rejection(code) => Self::Code(*code),
            Error::Invalid(violation) => Self::Invalid(Box::new(violation.clone())),
            _ => Self::Code(ErrorCode::MalformedMessage),
        }
    }

    pub(crate) fn code(&self) -> ErrorCode {
        match self {
            Self::Code(code) => *code,
            Self::Invalid(_) => ErrorCode::Invalid,
        }
    }

    pub(crate) fn error(&self) -> Error {
        match self {
            Self::Code(code) => Error::Rejection(*code),
            Self::Invalid(violation) => Error::Invalid((**violation).clone()),
        }
    }
}

impl From<ErrorCode> for Rejection {
    fn from(code: ErrorCode) -> Self {
        Self::Code(code)
    }
}
