use serde::Deserialize;

use super::template_option::TemplateOption;
use super::{NextMode, PayloadType};

/// `MissionConfig.CrossInfoSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct CrossInfoSpec {
    pub current_type: PayloadType,
    pub next_type: PayloadType,
    pub next_mode: NextMode,
    pub next_option: TemplateOption,
}
