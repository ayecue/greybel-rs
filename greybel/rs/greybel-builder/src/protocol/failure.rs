use std::fmt;

use super::BuildFailure;

impl BuildFailure {
    /// Creates an `Invalid` build failure containing `message` but no source location.
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid {
            message: message.into(),
            location: None,
        }
    }
}

/// Describes the failure without an `Invalid` failure's location; each host renders locations independently.
impl fmt::Display for BuildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { message, .. } | Self::Unreachable { message } => {
                formatter.write_str(message)
            }
            Self::Unavailable => {
                formatter.write_str("Grey Hack is not available in this environment.")
            }
        }
    }
}
