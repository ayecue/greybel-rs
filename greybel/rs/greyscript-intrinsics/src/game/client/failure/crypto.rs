//! Grey Hack failure text for `crypto.so`.

use crate::protocol::CryptoFailure;

/// Formats `CryptoIntrinsics` failures.
pub(super) const fn crypto_text(failure: CryptoFailure) -> &'static str {
    match failure {
        CryptoFailure::Encrypted => "error: file already encrypted",
        CryptoFailure::NotEncrypted => "decrypt: The file is not encrypted.",
        CryptoFailure::IncorrectPassword => "decrypt: Incorrect password",
        CryptoFailure::WifiOff => "wifi_networks: wifi card is disabled",
        CryptoFailure::NotWifiCard => "airmon: monitor mode can only be activated on wifi cards",
        CryptoFailure::NoMonitorSupport => {
            "airmon: monitor mode is not supported by the chipset of this network card."
        }
        CryptoFailure::OutOfRange => "Can't connect. Target is out of reach.",
        CryptoFailure::UnknownRouter => "router not found!",
        CryptoFailure::NoMonitorCard => "aireplay: no wifi card found with monitor mode enabled",
        CryptoFailure::UnreadableCapture => "Can't open file. Permission denied",
        CryptoFailure::InvalidCapture => "Can't process file. Not valid filecap.",
        CryptoFailure::EncryptedCapture => "Can't process file. File encrypted.",
        CryptoFailure::CorruptedCapture => "Error: filecap corrupted!",
        CryptoFailure::MoreAcksNeeded => "Key not found. More acks needed.",
    }
}
