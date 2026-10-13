use std::convert::Infallible;
use std::fmt::Debug;

use crate::definition::{ErrorCode, Violation};

/// Message encoding error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeFailure {
    /// A length exceeds its encoding range.
    #[error("a length is too large")]
    LengthOverflow,
    /// Contains the failed migration error code.
    #[error("{0}")]
    Migration(ErrorCode),
}

/// Message or response decoding error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeFailure {
    /// Input ended before the value was fully decoded.
    #[error("unexpected end of input")]
    InputEnd,
    /// Bytes remain after the decoded value.
    #[error("trailing input")]
    InputRemainder,
    /// Decoded integer is outside the target type range.
    #[error("an integer is too large")]
    IntegerOverflow,
    /// Encoded byte is not a valid Boolean.
    #[error("an invalid boolean")]
    InvalidBoolean,
    /// Encoded code point is not a valid character.
    #[error("an invalid character")]
    InvalidCharacter,
    /// Encoded flags include undeclared bits.
    #[error("invalid flags")]
    InvalidFlags,
    /// Encoded text is not valid UTF-8.
    #[error("invalid UTF-8")]
    InvalidUtf8,
    /// Input contains a failure for a request with no declared failure.
    #[error("a failure of a request that declares none")]
    UndeclaredFailure,
    /// Encountered a message frame while decoding a control frame.
    #[error("a message frame read as a control")]
    MessageAsControl,
    /// Acknowledgement lacks a connection ID.
    #[error("an acknowledgement of no connection")]
    EmptyConnection,
    /// Batch has no slices.
    #[error("a batch of no slices")]
    EmptyBatch,
    /// Input contains an undeclared error code.
    #[error("no error code {0}")]
    UnknownErrorCode(u32),
    /// Outer batch ends before an inner batch completes.
    #[error("the batches end inside a batch")]
    BatchEnd,
    /// Encoded batch exceeds the maximum byte length.
    #[error("a batch of {length} bytes is longer than {maximum}")]
    BatchSize {
        /// Encoded batch length in bytes.
        length: usize,
        /// Maximum allowed batch length in bytes.
        maximum: usize,
    },
    /// Batch length prefix exceeds the maximum byte count.
    #[error("a batch's length takes more than {maximum} bytes")]
    BatchLength {
        /// Maximum allowed length-prefix bytes.
        maximum: usize,
    },
    /// Response ended without a final event.
    #[error("the response didn't end")]
    ResponseEnd,
    /// Response ended before decoding its start value.
    #[error("the response ended without its start")]
    ResponseStart,
    /// Response item was decoded before the start value.
    #[error("an item of the response came before its start")]
    ResponseItemOrder,
    /// Input contains an undeclared union variant index.
    #[error("no variant {index} of `{name}`")]
    UnknownVariant {
        /// Input variant index.
        index: u32,
        /// Declared union name.
        name: &'static str,
    },
    /// Input contains an undeclared enumeration value.
    #[error("no member {value} of `{name}`")]
    UnknownMember {
        /// Input enumeration member value.
        value: u32,
        /// Declared enumeration name.
        name: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
/// Message exchange error.
pub enum Error<F: Debug = Infallible> {
    #[error("a message couldn't be encoded: {0}")]
    /// Encoding the message failed.
    Encode(EncodeFailure),
    #[error("a message couldn't be decoded: {0}")]
    /// Decoding the message or response failed.
    Decode(DecodeFailure),
    #[error("the message failed: {0:?}")]
    /// Remote peer returned a protocol-declared failure.
    Failure(F),
    #[error("{0}")]
    /// Remote peer rejected the message with an error code.
    Rejection(ErrorCode),
    #[error("{0}")]
    /// Channel closed with an error code.
    ChannelEnd(ErrorCode),
    #[error("a value doesn't meet its requirement: {0:?}")]
    /// Value or frame failed protocol validation.
    Invalid(Violation),
    #[error("the version the peers speak doesn't have it")]
    /// Negotiated protocol version lacks the message or variant.
    Unavailable,
}

impl Error {
    pub(crate) fn of_migration(code: ErrorCode) -> Self {
        match code {
            ErrorCode::UnsupportedVersion => Self::Unavailable,
            code => Self::Encode(EncodeFailure::Migration(code)),
        }
    }

    #[must_use]
    /// Converts the declared failure type to `F`.
    pub fn with_failure<F: Debug>(self) -> Error<F> {
        match self {
            Self::Encode(failure) => Error::Encode(failure),
            Self::Decode(failure) => Error::Decode(failure),
            Self::Failure(never) => match never {},
            Self::Rejection(code) => Error::Rejection(code),
            Self::ChannelEnd(code) => Error::ChannelEnd(code),
            Self::Invalid(violation) => Error::Invalid(violation),
            Self::Unavailable => Error::Unavailable,
        }
    }
}
