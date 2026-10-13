//! Conversions of package manager results.

use super::{AptFailure, AptUpdateLine};

impl From<greyhack_world::AptFailure> for AptFailure {
    fn from(failure: greyhack_world::AptFailure) -> Self {
        use greyhack_world::AptFailure as World;

        match failure {
            World::NoCache => Self::NoCache,
            World::InvalidCache => Self::InvalidCache,
            World::InvalidAddress(address) => Self::InvalidAddress(address),
            World::ListedRepository(address) => Self::ListedRepository(address),
            World::UnlistedRepository(address) => Self::UnlistedRepository(address),
            World::UnknownRepository(address) => Self::UnknownRepository(address),
            World::NoMatch(term) => Self::NoMatch(term),
            World::UnknownPackage(package) => Self::UnknownPackage(package),
            World::MissingPackageFile(package) => Self::MissingPackageFile(package),
            World::ComputerUnavailable => Self::ComputerUnavailable,
            World::MissingFolder(path) => Self::MissingFolder(path),
            World::NoPermission => Self::NoPermission,
            World::Copy(failure) => Self::Copy(failure.into()),
        }
    }
}

impl From<greyhack_world::AptUpdateLine> for AptUpdateLine {
    fn from(line: greyhack_world::AptUpdateLine) -> Self {
        match line {
            greyhack_world::AptUpdateLine::Update(source) => Self::Update(source),
            greyhack_world::AptUpdateLine::Connection(failure) => Self::Connection(failure.into()),
            greyhack_world::AptUpdateLine::Service(failure) => Self::Service(failure.into()),
            greyhack_world::AptUpdateLine::MissingConfig => Self::MissingConfig,
            greyhack_world::AptUpdateLine::MalformedConfig => Self::MalformedConfig,
        }
    }
}
