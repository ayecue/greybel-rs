use serde::Deserialize;

use super::EvidenceType;
use super::system_log_spec::SystemLogSpec;

/// `MissionConfig.EvidenceFunction`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct EvidenceFunction {
    #[serde(rename = "type")]
    pub kind: EvidenceType,
    pub force_account_on_current_node: bool,
    pub force_account_on_next_node: bool,
    /// `ServiceConfig.ServicioID` number.
    pub service: u8,
    pub http_website_text_id: String,
    pub system_log_spec: SystemLogSpec,
}
