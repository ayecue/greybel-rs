//! Protocol conversions for base values and failures. The implementations are compiled with the
//! base routes; downstream domains use the same conversion traits for their own values.

mod common;
mod computer;
mod files;
mod network;

pub use greyhack_world_router::protocol::*;

/// Converts a base world or behaviour value into its protocol representation.
pub trait IntoProtocol {
    /// Protocol value produced by the conversion.
    type Output;

    /// Performs the conversion.
    fn into_protocol(self) -> Self::Output;
}

/// Converts a base world value that can fail with a script failure.
pub trait TryProtocolFrom<S>: Sized {
    /// Script failure produced when the value has no protocol representation.
    type Error;

    /// Performs the fallible conversion.
    fn try_from(value: S) -> Result<Self, Self::Error>;
}

impl<T: IntoProtocol, E: IntoProtocol> IntoProtocol for Result<T, E> {
    type Output = Result<T::Output, E::Output>;

    fn into_protocol(self) -> Self::Output {
        self.map(IntoProtocol::into_protocol)
            .map_err(IntoProtocol::into_protocol)
    }
}

impl<T: IntoProtocol> IntoProtocol for Option<T> {
    type Output = Option<T::Output>;

    fn into_protocol(self) -> Self::Output {
        self.map(IntoProtocol::into_protocol)
    }
}

impl<T: IntoProtocol> IntoProtocol for Vec<T> {
    type Output = Vec<T::Output>;

    fn into_protocol(self) -> Self::Output {
        self.into_iter().map(IntoProtocol::into_protocol).collect()
    }
}
