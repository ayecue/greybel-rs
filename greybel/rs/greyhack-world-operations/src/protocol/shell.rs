//! Conversions of launch, import, and build failures.

use super::{ImportFailure, LaunchFailure, ScriptFailure, ShellFailure};

impl From<greyhack_world::LaunchFailure> for LaunchFailure {
    fn from(failure: greyhack_world::LaunchFailure) -> Self {
        use greyhack_world::LaunchFailure as World;

        match failure {
            World::NoCommand(path) => Self::CommandNotFound(path),
            World::NotExecutable(name) => Self::NotExecutable(name),
            World::NoPermission => Self::NoPermission,
            World::InvalidParameter(token) => Self::InvalidParameter(token),
            World::TooManyStackCalls => Self::TooManyStackCalls,
            World::Process(failure) => Self::Process(failure.into()),
        }
    }
}

impl From<greyhack_world::ImportFailure> for ImportFailure {
    fn from(failure: greyhack_world::ImportFailure) -> Self {
        use greyhack_world::ImportFailure as World;

        match failure {
            World::RelativePath(path) => Self::RelativePath(path),
            World::MissingFile(path) => Self::MissingFile(path),
            World::DuplicateImport(path) => Self::ImportedTwice(path),
            World::TooLarge => Self::TooLarge,
            World::BinaryFile(path) => Self::BinaryFile(path),
            World::NoImportPermission(path) => Self::NoImportPermission(path),
            World::Unreadable(path) => Self::Unreadable(path),
        }
    }
}

impl TryFrom<greyhack_world::BuildFailure> for ShellFailure {
    type Error = ScriptFailure;

    /// Converts a build failure; an unclosed `import_code(` crashes the game instead.
    fn try_from(failure: greyhack_world::BuildFailure) -> Result<Self, ScriptFailure> {
        use greyhack_world::BuildFailure as World;

        Ok(match failure {
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::MissingSource(path) => Self::MissingSource(path),
            World::MissingFolder(path) => Self::MissingFolder(path),
            World::UnreadableSource(path) => Self::UnreadableSource(path),
            World::BinarySource(name) => Self::BinarySource(name),
            World::UnwritableFolder(path) => Self::UnwritableFolder(path),
            World::InvalidExtension(name) => Self::InvalidExtension(name),
            World::EmptySource => Self::EmptySource,
            World::Import(failure) => Self::Import(failure.into()),
            World::UnclosedImport => return Err(ScriptFailure::GameCrash),
        })
    }
}
