//! Conversions of file failures.

use greyhack_world_programs::protocol::{Failure, FileFailure, ScriptFailure};

use super::{IntoProtocol, TryProtocolFrom};

impl IntoProtocol for crate::FileFailure {
    type Output = FileFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::FileFailure as World;

        match failure {
            World::ComputerUnavailable => FileFailure::ComputerUnavailable,
            World::MissingFile(path) => FileFailure::MissingFile(path),
            World::MissingFolder(path) => FileFailure::MissingFolder(path),
            World::NotAFolder(path) => FileFailure::NotAFolder(path),
            World::InvalidPath => FileFailure::InvalidPath,
            World::NoPermission => FileFailure::NoPermission,
            World::SystemFile => FileFailure::SystemFile,
            World::SystemFileTarget(name) => FileFailure::SystemFileTarget(name),
            World::KindMismatch => FileFailure::KindMismatch,
            World::DiskFull => FileFailure::DiskFull,
            World::FolderFull => FileFailure::FolderFull,
            World::TooManyFiles => FileFailure::TooManyFiles,
            World::WrongFormat => FileFailure::WrongFormat,
            World::CannotAccess(path) => FileFailure::CannotAccess(path),
            World::CannotRead(path) => FileFailure::CannotRead(path),
            World::CannotWrite(path) => FileFailure::CannotWrite(path),
            World::CannotReplace(path) => FileFailure::CannotReplace(path),
            World::OverwriteSelf => FileFailure::OverwriteSelf,
            World::PartialCopy => FileFailure::PartialCopy,
            World::PartialDelete => FileFailure::PartialDelete,
            World::DeletePermission(path) => FileFailure::DeletePermission(path),
            World::FolderReadPermission(path) => FileFailure::FolderReadPermission(path),
        }
    }
}

impl TryProtocolFrom<crate::ScriptFileFailure> for Failure {
    type Error = ScriptFailure;

    /// Converts a file operation failure; a move or symlink path with too many folders is a script
    /// failure instead.
    fn try_from(failure: crate::ScriptFileFailure) -> Result<Self, ScriptFailure> {
        use crate::ScriptFileFailure as World;

        Ok(Self::File(match failure {
            World::File(failure) => failure.into_protocol(),
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
            World::Refusal(refusal) => return Ok(Self::Refusal(refusal.into_protocol())),
        }))
    }
}
