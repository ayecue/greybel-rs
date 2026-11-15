//! Conversions of package manager results.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_programs::protocol::{AptFailure, AptUpdateLine};

impl IntoProtocol for crate::AptFailure {
    type Output = AptFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::AptFailure as World;

        match failure {
            World::NoCache => AptFailure::NoCache,
            World::InvalidCache => AptFailure::InvalidCache,
            World::InvalidAddress(address) => AptFailure::InvalidAddress(address),
            World::ListedRepository(address) => AptFailure::ListedRepository(address),
            World::UnlistedRepository(address) => AptFailure::UnlistedRepository(address),
            World::UnknownRepository(address) => AptFailure::UnknownRepository(address),
            World::NoMatch(term) => AptFailure::NoMatch(term),
            World::UnknownPackage(package) => AptFailure::UnknownPackage(package),
            World::MissingPackageFile(package) => AptFailure::MissingPackageFile(package),
            World::ComputerUnavailable => AptFailure::ComputerUnavailable,
            World::MissingFolder(path) => AptFailure::MissingFolder(path),
            World::NoPermission => AptFailure::NoPermission,
            World::Copy(failure) => AptFailure::Copy(failure.into_protocol()),
            World::NoInternet => AptFailure::NoInternet,
            World::EmptyPackageName => AptFailure::EmptyPackageName,
            World::MalformedCache => AptFailure::MalformedCache,
            World::MissingLocalFile(name) => AptFailure::MissingLocalFile(name),
            World::Connection(failure) => AptFailure::Connection(failure.into_protocol()),
            World::Service(failure) => AptFailure::Service(failure.into_protocol()),
            World::MissingConfig => AptFailure::MissingConfig,
            World::MissingRepositoryFolder => AptFailure::MissingRepositoryFolder,
        }
    }
}

impl IntoProtocol for crate::AptUpdateLine {
    type Output = AptUpdateLine;

    fn into_protocol(self) -> Self::Output {
        let line = self;

        match line {
            crate::AptUpdateLine::Update(source) => AptUpdateLine::Update(source),
            crate::AptUpdateLine::Connection(failure) => {
                AptUpdateLine::Connection(failure.into_protocol())
            }
            crate::AptUpdateLine::Service(failure) => {
                AptUpdateLine::Service(failure.into_protocol())
            }
            crate::AptUpdateLine::MissingConfig => AptUpdateLine::MissingConfig,
            crate::AptUpdateLine::MalformedConfig => AptUpdateLine::MalformedConfig,
        }
    }
}
