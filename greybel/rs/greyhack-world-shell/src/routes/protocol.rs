//! Conversions of launch, import, and build failures.

pub(crate) use greyhack_world_core::routes::conversions::{IntoProtocol, TryProtocolFrom};
pub(crate) use greyhack_world_programs::protocol::*;

impl IntoProtocol for crate::LaunchFailure {
    type Output = LaunchFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::LaunchFailure::NoCommand(path) => LaunchFailure::CommandNotFound(path),
            crate::LaunchFailure::NotExecutable(name) => LaunchFailure::NotExecutable(name),
            crate::LaunchFailure::NoPermission => LaunchFailure::NoPermission,
            crate::LaunchFailure::InvalidParameter(token) => LaunchFailure::InvalidParameter(token),
            crate::LaunchFailure::TooManyStackCalls => LaunchFailure::TooManyStackCalls,
            crate::LaunchFailure::Process(failure) => LaunchFailure::Process(
                greyhack_world_core::routes::conversions::IntoProtocol::into_protocol(failure),
            ),
        }
    }
}

impl IntoProtocol for crate::ImportFailure {
    type Output = ImportFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::ImportFailure::RelativePath(path) => ImportFailure::RelativePath(path),
            crate::ImportFailure::MissingFile(path) => ImportFailure::MissingFile(path),
            crate::ImportFailure::DuplicateImport(path) => ImportFailure::ImportedTwice(path),
            crate::ImportFailure::TooLarge => ImportFailure::TooLarge,
            crate::ImportFailure::BinaryFile(path) => ImportFailure::BinaryFile(path),
            crate::ImportFailure::NoImportPermission(path) => {
                ImportFailure::NoImportPermission(path)
            }
            crate::ImportFailure::Unreadable(path) => ImportFailure::Unreadable(path),
        }
    }
}

impl TryProtocolFrom<crate::BuildFailure> for ShellFailure {
    type Error = ScriptFailure;

    /// Converts a build failure; an unclosed `import_code(` crashes the game instead.
    fn try_from(failure: crate::BuildFailure) -> Result<Self, ScriptFailure> {
        Ok(match failure {
            crate::BuildFailure::ComputerUnavailable => Self::ComputerUnavailable,
            crate::BuildFailure::MissingSource(path) => Self::MissingSource(path),
            crate::BuildFailure::MissingFolder(path) => Self::MissingFolder(path),
            crate::BuildFailure::UnreadableSource(path) => Self::UnreadableSource(path),
            crate::BuildFailure::BinarySource(name) => Self::BinarySource(name),
            crate::BuildFailure::UnwritableFolder(path) => Self::UnwritableFolder(path),
            crate::BuildFailure::InvalidExtension(name) => Self::InvalidExtension(name),
            crate::BuildFailure::EmptySource => Self::EmptySource,
            crate::BuildFailure::Import(failure) => Self::Import(failure.into_protocol()),
            crate::BuildFailure::UnclosedImport => return Err(ScriptFailure::GameCrash),
        })
    }
}
