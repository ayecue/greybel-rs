//! Failures of script file operations (`FileIntrinsics`, `PlayerUtils.CrearArchivo`).

use greyhack_behaviour::Refusal;

use crate::FileFailure;

/// Why a file operation a script started failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptFileFailure {
    /// A failure the file system rules report.
    File(FileFailure),
    /// An entry of the name already exists; contains its name.
    Exists(String),
    /// The destination path is empty.
    EmptyPath,
    /// The user cannot create a file of the name in the folder.
    NoCreatePermission {
        /// Folder path.
        folder: String,
        /// Name of the file.
        name: String,
    },
    /// The file system contains the maximum number of files for a new folder.
    TooManyFolders,
    /// The path exceeds the maximum folder nesting depth.
    TooManySubfolders,
    /// The folder contains an entry of the new name already.
    NameExists,
    /// The root folder cannot be moved.
    RootMove,
    /// The symlink would be at the path of its target.
    SymlinkToItself,
    /// The file at the path is binary; contains the path.
    Binary(String),
    /// The user cannot read the file at the path.
    NoReadPermission(String),
    /// The folder path of a move exceeds the maximum number of folders.
    TooManyPathFolders,
    /// An argument breaks a rule.
    Refusal(Refusal),
}

impl From<FileFailure> for ScriptFileFailure {
    fn from(failure: FileFailure) -> Self {
        Self::File(failure)
    }
}

impl From<Refusal> for ScriptFileFailure {
    fn from(refusal: Refusal) -> Self {
        Self::Refusal(refusal)
    }
}
