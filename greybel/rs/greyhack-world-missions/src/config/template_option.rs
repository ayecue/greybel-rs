use serde::Deserialize;

use super::EvidenceType;
use super::chat_log_path_spec::ChatLogPathSpec;
use super::cross_info_spec::CrossInfoSpec;
use super::evidence_function::EvidenceFunction;
use super::file_option_spec::FileOptionSpec;

/// `MissionConfig.TemplateOption`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TemplateOption {
    pub group: String,
    pub text_id: String,
    pub email_title_text_id: String,
    pub evidence: Vec<EvidenceFunction>,
    pub file_spec: FileOptionSpec,
    pub chat_log_path_spec: ChatLogPathSpec,
    pub generate_support_chat_log: bool,
    pub support_chat_log_text_id: String,
    pub support_chat_log_path_spec: ChatLogPathSpec,
    pub cross_info_spec: Option<Box<CrossInfoSpec>>,
}

impl TemplateOption {
    /// `ProcMissionsDefaults.EnsureTemplateOptionShape`.
    pub(super) fn ensure_shape(&mut self) {
        self.group = self.group.trim().to_owned();

        if let Some(spec) = &mut self.cross_info_spec {
            spec.next_option.ensure_shape();
        }
    }

    /// `ProcMissionBuilder.ContainsEvidence`.
    pub fn contains_evidence(&self, kind: EvidenceType) -> bool {
        self.evidence.iter().any(|function| function.kind == kind)
    }
}
