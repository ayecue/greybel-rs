use serde::Deserialize;

use super::closure_entry::ClosureEntry;

/// `MissionConfig.ClosureConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ClosureConfig {
    pub entries: Vec<ClosureEntry>,
}
