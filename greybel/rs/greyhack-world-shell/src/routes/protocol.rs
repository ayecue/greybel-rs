//! Conversions of launch, import, and build failures.

pub(crate) use greyhack_world_core::routes::conversions::{IntoProtocol, TryProtocolFrom};
pub(crate) use greyhack_world_router::protocol::*;

impl IntoProtocol for crate::LaunchFailure {
    type Output = LaunchFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::LaunchFailure as World;

        match failure {
            World::NoCommand(path) => LaunchFailure::CommandNotFound(path),
            World::NotExecutable(name) => LaunchFailure::NotExecutable(name),
            World::NoPermission => LaunchFailure::NoPermission,
            World::InvalidParameter(token) => LaunchFailure::InvalidParameter(token),
            World::TooManyStackCalls => LaunchFailure::TooManyStackCalls,
            World::Process(failure) => LaunchFailure::Process(
                greyhack_world_core::routes::conversions::IntoProtocol::into_protocol(failure),
            ),
        }
    }
}

impl IntoProtocol for crate::ImportFailure {
    type Output = ImportFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ImportFailure as World;

        match failure {
            World::RelativePath(path) => ImportFailure::RelativePath(path),
            World::MissingFile(path) => ImportFailure::MissingFile(path),
            World::DuplicateImport(path) => ImportFailure::ImportedTwice(path),
            World::TooLarge => ImportFailure::TooLarge,
            World::BinaryFile(path) => ImportFailure::BinaryFile(path),
            World::NoImportPermission(path) => ImportFailure::NoImportPermission(path),
            World::Unreadable(path) => ImportFailure::Unreadable(path),
        }
    }
}

impl TryProtocolFrom<crate::BuildFailure> for ShellFailure {
    type Error = ScriptFailure;

    /// Converts a build failure; an unclosed `import_code(` crashes the game instead.
    fn try_from(failure: crate::BuildFailure) -> Result<Self, ScriptFailure> {
        use crate::BuildFailure as World;

        Ok(match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::MissingSource(path) => Self::MissingSource(path),
            World::MissingFolder(path) => Self::MissingFolder(path),
            World::UnreadableSource(path) => Self::UnreadableSource(path),
            World::BinarySource(name) => Self::BinarySource(name),
            World::UnwritableFolder(path) => Self::UnwritableFolder(path),
            World::InvalidExtension(name) => Self::InvalidExtension(name),
            World::EmptySource => Self::EmptySource,
            World::Import(failure) => Self::Import(failure.into_protocol()),
            World::UnclosedImport => return Err(ScriptFailure::GameCrash),
        })
    }
}
