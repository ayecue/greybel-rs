//! Wi-Fi traffic captured by `aireplay`, serialized as the JSON capture file read by `aircrack`.

use greyhack_behaviour::required_acks;
use serde::{Deserialize, Serialize};

/// Router Wi-Fi capture with Grey Hack field names (`PlayerUtils.HelperAireplay`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiCapture {
    /// Router signal power.
    pub power: i32,
    #[serde(rename = "numAcks")]
    pub num_acks: i32,
    pub bssid: String,
    pub essid: String,
    /// Router public address.
    #[serde(rename = "origPublicIp")]
    pub public_ip: String,
}

impl WifiCapture {
    /// Parses capture-file contents. Returns `None` for corrupted data.
    #[must_use]
    pub fn parse(content: &str) -> Option<Self> {
        serde_json::from_str(content).ok()
    }

    /// Serialized capture-file contents.
    #[must_use]
    pub fn to_content(&self) -> String {
        serde_json::to_string(self).expect("a capture serializes")
    }

    /// Returns whether the capture contains the ACKs required by `aircrack` to recover the key.
    #[must_use]
    pub const fn has_enough_acks(&self) -> bool {
        self.num_acks >= required_acks(self.power)
    }
}
