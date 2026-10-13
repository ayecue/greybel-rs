use serde::Deserialize;

use super::ClosureType;
use super::template_option::TemplateOption;

/// `MissionConfig.ClosureEntry`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ClosureEntry {
    #[serde(rename = "type")]
    pub kind: ClosureType,
    pub option: TemplateOption,
}
