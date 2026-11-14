//! Conversions of crypto failures.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
pub(crate) use greyhack_world_router::protocol::*;

impl IntoProtocol for crate::CryptoFailure {
    type Output = Failure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::CryptoFailure as World;

        Failure::Crypto(match failure {
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
            World::File(failure) => return Failure::File(failure.into_protocol()),
            World::Refusal(refusal) => return Failure::Refusal(refusal.into_protocol()),
        })
    }
}
