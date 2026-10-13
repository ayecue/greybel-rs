use serde::Deserialize;

use super::verb::VerbConfig;

/// `MissionConfig.HeartConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct HeartConfig {
    pub verbs: Vec<VerbConfig>,
}
