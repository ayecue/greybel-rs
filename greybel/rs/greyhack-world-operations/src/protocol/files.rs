//! Conversions of file failures.

use super::{Failure, FileFailure, ScriptFailure};

impl From<greyhack_world::FileFailure> for FileFailure {
    fn from(failure: greyhack_world::FileFailure) -> Self {
        use greyhack_world::FileFailure as World;

        match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::MissingFile(path) => Self::MissingFile(path),
            World::MissingFolder(path) => Self::MissingFolder(path),
            World::NotAFolder(path) => Self::NotAFolder(path),
            World::InvalidPath => Self::InvalidPath,
            World::NoPermission => Self::NoPermission,
            World::SystemFile => Self::SystemFile,
            World::SystemFileTarget(name) => Self::SystemFileTarget(name),
            World::KindMismatch => Self::KindMismatch,
            World::DiskFull => Self::DiskFull,
            World::FolderFull => Self::FolderFull,
            World::TooManyFiles => Self::TooManyFiles,
            World::WrongFormat => Self::WrongFormat,
            World::CannotAccess(path) => Self::CannotAccess(path),
            World::CannotRead(path) => Self::CannotRead(path),
            World::CannotWrite(path) => Self::CannotWrite(path),
            World::CannotReplace(path) => Self::CannotReplace(path),
            World::OverwriteSelf => Self::OverwriteSelf,
            World::PartialCopy => Self::PartialCopy,
            World::PartialDelete => Self::PartialDelete,
            World::DeletePermission(path) => Self::DeletePermission(path),
            World::FolderReadPermission(path) => Self::FolderReadPermission(path),
        }
    }
}

impl TryFrom<greyhack_world::ScriptFileFailure> for Failure {
    type Error = ScriptFailure;

    /// Converts a file operation failure; a move or symlink path with too many folders is a script
    /// failure instead.
    fn try_from(failure: greyhack_world::ScriptFileFailure) -> Result<Self, ScriptFailure> {
        use greyhack_world::ScriptFileFailure as World;

        Ok(Self::File(match failure {
            World::File(failure) => failure.into(),
            World::Exists(name) => FileFailure::Exists(name),
            World::EmptyPath => FileFailure::EmptyPath,
            World::NoCreatePermission { folder, name } => {
                FileFailure::NoCreatePermission { folder, name }
            }
            World::TooManyFolders => FileFailure::TooManyFolders,
            World::TooManySubfolders => FileFailure::TooManySubfolders,
            World::NameExists => FileFailure::NameTaken,
            World::RootMove => FileFailure::RootMove,
            World::SymlinkToItself => FileFailure::SymlinkToItself,
            World::Binary(path) => FileFailure::Binary(path),
            World::NoReadPermission(path) => FileFailure::NoReadPermission(path),
            World::TooManyPathFolders => return Err(ScriptFailure::TooManyFolders),
            World::Refusal(refusal) => return Ok(Self::Refusal(refusal.into())),
        }))
    }
}
