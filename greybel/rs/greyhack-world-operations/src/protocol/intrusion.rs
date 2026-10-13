//! Conversions of crypto failures.

use super::{CryptoFailure, Failure};

impl From<greyhack_world::CryptoFailure> for Failure {
    fn from(failure: greyhack_world::CryptoFailure) -> Self {
        use greyhack_world::CryptoFailure as World;

        Self::Crypto(match failure {
            World::HasEncryption => CryptoFailure::Encrypted,
            World::NoEncryption => CryptoFailure::NotEncrypted,
            World::IncorrectPassword => CryptoFailure::IncorrectPassword,
            World::NotWifiCard => CryptoFailure::NotWifiCard,
            World::NoMonitorSupport => CryptoFailure::NoMonitorSupport,
            World::UnknownRouter => CryptoFailure::UnknownRouter,
            World::NoMonitorCard => CryptoFailure::NoMonitorCard,
            World::UnreadableCapture => CryptoFailure::UnreadableCapture,
            World::InvalidCapture => CryptoFailure::InvalidCapture,
            World::EncryptedCapture => CryptoFailure::EncryptedCapture,
            World::CorruptedCapture => CryptoFailure::CorruptedCapture,
            World::MoreAcks => CryptoFailure::MoreAcksNeeded,
            World::File(failure) => return Self::File(failure.into()),
            World::Refusal(refusal) => return Self::Refusal(refusal.into()),
        })
    }
}
