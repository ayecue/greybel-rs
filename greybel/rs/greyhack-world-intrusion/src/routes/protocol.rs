//! Conversions of crypto failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_programs::protocol::*;

impl IntoProtocol for crate::CryptoFailure {
    type Output = Failure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        Failure::Crypto(match failure {
            crate::CryptoFailure::HasEncryption => CryptoFailure::Encrypted,
            crate::CryptoFailure::NoEncryption => CryptoFailure::NotEncrypted,
            crate::CryptoFailure::IncorrectPassword => CryptoFailure::IncorrectPassword,
            crate::CryptoFailure::WifiOff => CryptoFailure::WifiOff,
            crate::CryptoFailure::NotWifiCard => CryptoFailure::NotWifiCard,
            crate::CryptoFailure::NoMonitorSupport => CryptoFailure::NoMonitorSupport,
            crate::CryptoFailure::OutOfRange => CryptoFailure::OutOfRange,
            crate::CryptoFailure::UnknownRouter => CryptoFailure::UnknownRouter,
            crate::CryptoFailure::NoMonitorCard => CryptoFailure::NoMonitorCard,
            crate::CryptoFailure::UnreadableCapture => CryptoFailure::UnreadableCapture,
            crate::CryptoFailure::InvalidCapture => CryptoFailure::InvalidCapture,
            crate::CryptoFailure::EncryptedCapture => CryptoFailure::EncryptedCapture,
            crate::CryptoFailure::CorruptedCapture => CryptoFailure::CorruptedCapture,
            crate::CryptoFailure::MoreAcks => CryptoFailure::MoreAcksNeeded,
            crate::CryptoFailure::File(failure) => return Failure::File(failure.into_protocol()),
            crate::CryptoFailure::Refusal(refusal) => {
                return Failure::Refusal(refusal.into_protocol());
            }
        })
    }
}
