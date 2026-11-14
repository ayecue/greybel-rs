use serde::Deserialize;

use super::defaults::yes;

/// `MissionConfig.ChatLogPathSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ChatLogPathSpec {
    pub random_path: bool,
    pub path: String,
}

impl Default for ChatLogPathSpec {
    fn default() -> Self {
        Self {
            random_path: yes(),
            path: String::new(),
        }
    }
}
