use serde::Deserialize;

use super::Verb;
use super::modality::ModalityConfig;

/// `MissionConfig.VerbConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct VerbConfig {
    pub verb: Verb,
    pub modalities: Vec<ModalityConfig>,
}
