//! Conversions of file failures.

use greyhack_world_programs::protocol::{Failure, FileFailure, ScriptFailure};

use super::{IntoProtocol, TryProtocolFrom};

impl IntoProtocol for crate::FileFailure {
    type Output = FileFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::FileFailure::ComputerUnavailable => FileFailure::ComputerUnavailable,
            crate::FileFailure::MissingFile(path) => FileFailure::MissingFile(path),
            crate::FileFailure::MissingFolder(path) => FileFailure::MissingFolder(path),
            crate::FileFailure::NotAFolder(path) => FileFailure::NotAFolder(path),
            crate::FileFailure::InvalidPath => FileFailure::InvalidPath,
            crate::FileFailure::NoPermission => FileFailure::NoPermission,
            crate::FileFailure::SystemFile => FileFailure::SystemFile,
            crate::FileFailure::SystemFileTarget(name) => FileFailure::SystemFileTarget(name),
            crate::FileFailure::KindMismatch => FileFailure::KindMismatch,
            crate::FileFailure::DiskFull => FileFailure::DiskFull,
            crate::FileFailure::FolderFull => FileFailure::FolderFull,
            crate::FileFailure::TooManyFiles => FileFailure::TooManyFiles,
            crate::FileFailure::WrongFormat => FileFailure::WrongFormat,
            crate::FileFailure::CannotAccess(path) => FileFailure::CannotAccess(path),
            crate::FileFailure::CannotRead(path) => FileFailure::CannotRead(path),
            crate::FileFailure::CannotWrite(path) => FileFailure::CannotWrite(path),
            crate::FileFailure::CannotReplace(path) => FileFailure::CannotReplace(path),
            crate::FileFailure::OverwriteSelf => FileFailure::OverwriteSelf,
            crate::FileFailure::PartialCopy => FileFailure::PartialCopy,
            crate::FileFailure::PartialDelete => FileFailure::PartialDelete,
            crate::FileFailure::DeletePermission(path) => FileFailure::DeletePermission(path),
            crate::FileFailure::FolderReadPermission(path) => {
                FileFailure::FolderReadPermission(path)
            }
        }
    }
}

impl TryProtocolFrom<crate::ScriptFileFailure> for Failure {
    type Error = ScriptFailure;

    /// Converts a file operation failure; a move or symlink path with too many folders is a script
    /// failure instead.
    fn try_from(failure: crate::ScriptFileFailure) -> Result<Self, ScriptFailure> {
        Ok(Self::File(match failure {
            crate::ScriptFileFailure::File(failure) => failure.into_protocol(),
            crate::ScriptFileFailure::Exists(name) => FileFailure::Exists(name),
            crate::ScriptFileFailure::EmptyPath => FileFailure::EmptyPath,
            crate::ScriptFileFailure::NoCreatePermission { folder, name } => {
                FileFailure::NoCreatePermission { folder, name }
            }
            crate::ScriptFileFailure::TooManyFolders => FileFailure::TooManyFolders,
            crate::ScriptFileFailure::TooManySubfolders => FileFailure::TooManySubfolders,
            crate::ScriptFileFailure::NameExists => FileFailure::NameTaken,
            crate::ScriptFileFailure::RootMove => FileFailure::RootMove,
            crate::ScriptFileFailure::SymlinkToItself => FileFailure::SymlinkToItself,
            crate::ScriptFileFailure::Binary(path) => FileFailure::Binary(path),
            crate::ScriptFileFailure::NoReadPermission(path) => FileFailure::NoReadPermission(path),
            crate::ScriptFileFailure::TooManyPathFolders => {
                return Err(ScriptFailure::TooManyFolders);
            }
            crate::ScriptFileFailure::Refusal(refusal) => {
                return Ok(Self::Refusal(refusal.into_protocol()));
            }
        }))
    }
}
