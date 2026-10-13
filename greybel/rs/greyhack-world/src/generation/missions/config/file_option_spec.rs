use serde::Deserialize;

use super::companion_file_spec::CompanionFileSpec;
use super::defaults::{default_file_name, default_raw_content, yes};

/// `MissionConfig.FileOptionSpec`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct FileOptionSpec {
    pub random_file_name: bool,
    pub file_name: String,
    pub random_path: bool,
    pub path: String,
    pub symlink_target: String,
    pub owner: String,
    pub permissions: String,
    pub directory_permissions: String,
    pub encrypt_with_owner_password: bool,
    pub encryption_password: String,
    pub is_pdf: bool,
    pub is_image: bool,
    pub is_binary: bool,
    pub is_script: bool,
    pub allow_import: bool,
    pub companion_files: Vec<CompanionFileSpec>,
    pub raw_content: String,
}

impl Default for FileOptionSpec {
    fn default() -> Self {
        Self {
            random_file_name: yes(),
            file_name: default_file_name(),
            random_path: yes(),
            path: String::new(),
            symlink_target: String::new(),
            owner: String::new(),
            permissions: String::new(),
            directory_permissions: String::new(),
            encrypt_with_owner_password: false,
            encryption_password: String::new(),
            is_pdf: false,
            is_image: false,
            is_binary: false,
            is_script: false,
            allow_import: false,
            companion_files: Vec::new(),
            raw_content: default_raw_content(),
        }
    }
}
