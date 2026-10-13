use std::fmt::{self, Display, Formatter};

use crate::definition::ErrorCode;
use crate::error::Error;

impl ErrorCode {
    #[doc(hidden)]
    #[must_use]
    pub fn of_migration(error: &Error) -> Self {
        match error {
            Error::Unavailable => Self::UnsupportedVersion,
            _ => Self::MalformedMessage,
        }
    }

    #[must_use]
    /// Returns the display message for this error code.
    pub fn message(self) -> &'static str {
        match self {
            Self::ClosedChannel => "the channel closed",
            Self::Timeout => "the connection timed out",
            Self::VersionMismatch => {
                "the client and the provider speak versions that can't read each other"
            }
            Self::Busy => "the channel is already connected",
            Self::ConnectionLoss => "the connection to the other peer was lost",
            Self::Panic => "a peer failed",
            Self::UnhandledMessage => "no handler took the message",
            Self::DroppedMessage => "the message got no response",
            Self::HandlerFailure => "the handler of the message failed",
            Self::MalformedMessage => "a message couldn't be written or read",
            Self::UnsupportedVersion => "the version isn't supported",
            Self::Denial => "the provider denied the client",
            Self::Cancellation => "the sender cancelled the message",
            Self::Insecure => "the transport isn't secure enough",
            Self::Overload => "a peer reached its limits",
            Self::Invalid => "a value doesn't meet what its definition requires",
        }
    }
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}
