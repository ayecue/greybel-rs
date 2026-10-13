//! JSON camera feed exposed through the browser CCTV channel (`NetworkHelperServer` CCTV access).

use serde::Serialize;

/// Camera feed containing the camera and people on its network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CameraFeed {
    /// Camera LAN address.
    pub local_ip: String,
    pub npcs: Vec<CameraNpc>,
}

/// Person visible on the camera network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CameraNpc {
    pub name: String,
    pub online: bool,
}

impl CameraFeed {
    /// Serialized JSON feed.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a camera feed serializes")
    }
}
