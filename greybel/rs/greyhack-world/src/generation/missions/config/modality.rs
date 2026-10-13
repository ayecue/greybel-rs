use serde::Deserialize;

use super::Modality;
use super::template_option::TemplateOption;

/// `MissionConfig.ModalityConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct ModalityConfig {
    pub modality: Modality,
    pub options: Vec<TemplateOption>,
}
