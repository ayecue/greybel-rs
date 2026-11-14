use crate::config::CompanionFileSpec;

/// `MissionConfig.FileItem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileItem {
    pub row_shuffle_seed: i32,
    pub file_name: String,
    pub random_path: bool,
    pub is_pdf: bool,
    pub is_binary: bool,
    pub is_script: bool,
    pub allow_import: bool,
    pub companion_files: Vec<CompanionFileSpec>,
    pub is_image: bool,
    pub path: String,
    pub symlink_target: String,
    pub owner: String,
    pub permissions: String,
    pub directory_permissions: String,
    pub encrypt_with_owner_password: bool,
    pub encryption_password: String,
    /// The text of the item is a game text file (`XmlGlobal.CacheText.textFiles`), such as a website.
    pub is_web: bool,
    pub raw_content: String,
}
