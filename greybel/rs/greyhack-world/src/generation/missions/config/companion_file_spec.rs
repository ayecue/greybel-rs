use serde::Deserialize;

/// `MissionConfig.CompanionFileSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct CompanionFileSpec {
    pub file_name: String,
    pub content: String,
    pub text_id: String,
    pub permissions: String,
    pub encryption_password: String,
}
