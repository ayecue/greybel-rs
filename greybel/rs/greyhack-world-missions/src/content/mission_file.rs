use greyhack_behaviour::FileData;

/// A file or folder permission change a mission makes, with where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionFile {
    /// Folder the file is added to (`FileSystem.AddFichero(file, path)`).
    pub folder: String,
    pub name: String,
    pub data: FileData,
    pub owner: String,
    pub permissions: Option<String>,
    pub symlink: String,
    pub timestamp: Option<u64>,
    /// Permissions to give the folder, which is created when missing (`FileSystem.GetLastFolder`).
    pub folder_permissions: Option<String>,
    /// Whether the folder is created when missing (objective files) rather than the file skipped.
    pub create_folder: bool,
}
