use serde::Deserialize;

use super::defaults::default_port;

/// `MissionConfig.SystemLogSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct SystemLogSpec {
    /// `LogSystem.StatusLog` number.
    pub entry_type: u8,
    pub port: i32,
    pub is_receiving_conn: bool,
}

impl Default for SystemLogSpec {
    fn default() -> Self {
        Self {
            entry_type: 0,
            port: default_port(),
            is_receiving_conn: false,
        }
    }
}
