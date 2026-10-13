use std::fmt;

use super::{EvaluationFailure, LaunchFailure};

impl LaunchFailure {
    /// Creates an `Invalid` launch failure containing `message` but no source location.
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            location: None,
        }
    }

    /// Creates an `Unreachable` launch failure describing why Grey Hack cannot execute the program.
    #[cfg(feature = "game")]
    pub(crate) fn unreachable(message: impl Into<String>) -> Self {
        Self::Unreachable {
            message: message.into(),
        }
    }
}

/// Describes the failure without an `Invalid` failure's location; hosts render locations independently.
impl fmt::Display for LaunchFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { message, .. } | Self::Unreachable { message } => {
                formatter.write_str(message)
            }
            Self::Unavailable => formatter.write_str("Grey Hack can't be reached from here."),
            Self::Busy => formatter.write_str("A program runs already."),
        }
    }
}

impl EvaluationFailure {
    /// Creates a `CodeError` evaluation failure containing `message`.
    pub(crate) fn from_message(message: impl Into<String>) -> Self {
        Self::CodeError {
            message: message.into(),
        }
    }
}

impl fmt::Display for EvaluationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CodeError { message } => formatter.write_str(message),
            Self::ActiveProgram => formatter.write_str("The program isn't stopped."),
        }
    }
}
