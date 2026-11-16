//! Conversions of package manager results.

pub(crate) use greyhack_world_core::routes::conversions::IntoProtocol;
use greyhack_world_programs::protocol::{AptFailure, AptUpdateLine};

impl IntoProtocol for crate::AptFailure {
    type Output = AptFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::AptFailure::NoCache => AptFailure::NoCache,
            crate::AptFailure::InvalidCache => AptFailure::InvalidCache,
            crate::AptFailure::InvalidAddress(address) => AptFailure::InvalidAddress(address),
            crate::AptFailure::ListedRepository(address) => AptFailure::ListedRepository(address),
            crate::AptFailure::UnlistedRepository(address) => {
                AptFailure::UnlistedRepository(address)
            }
            crate::AptFailure::UnknownRepository(address) => AptFailure::UnknownRepository(address),
            crate::AptFailure::NoMatch(term) => AptFailure::NoMatch(term),
            crate::AptFailure::UnknownPackage(package) => AptFailure::UnknownPackage(package),
            crate::AptFailure::MissingPackageFile(package) => {
                AptFailure::MissingPackageFile(package)
            }
            crate::AptFailure::ComputerUnavailable => AptFailure::ComputerUnavailable,
            crate::AptFailure::MissingFolder(path) => AptFailure::MissingFolder(path),
            crate::AptFailure::NoSpace => AptFailure::NoSpace,
            crate::AptFailure::NoPermission => AptFailure::NoPermission,
            crate::AptFailure::Copy(failure) => AptFailure::Copy(failure.into_protocol()),
            crate::AptFailure::NoInternet => AptFailure::NoInternet,
            crate::AptFailure::EmptyPackageName => AptFailure::EmptyPackageName,
            crate::AptFailure::MissingSources => AptFailure::MissingSources,
            crate::AptFailure::BinarySources => AptFailure::BinarySources,
            crate::AptFailure::MalformedSources => AptFailure::MalformedSources,
            crate::AptFailure::MalformedCache => AptFailure::MalformedCache,
            crate::AptFailure::MissingLocalFile(name) => AptFailure::MissingLocalFile(name),
            crate::AptFailure::Connection(failure) => {
                AptFailure::Connection(failure.into_protocol())
            }
            crate::AptFailure::Service(failure) => AptFailure::Service(failure.into_protocol()),
            crate::AptFailure::MissingConfig => AptFailure::MissingConfig,
            crate::AptFailure::MalformedConfig => AptFailure::MalformedConfig,
            crate::AptFailure::MalformedConfigUpgrade(package) => {
                AptFailure::MalformedConfigUpgrade(package)
            }
            crate::AptFailure::MissingRepositoryFolder => AptFailure::MissingRepositoryFolder,
        }
    }
}

impl IntoProtocol for crate::AptUpdateLine {
    type Output = AptUpdateLine;

    fn into_protocol(self) -> Self::Output {
        let line = self;

        match line {
            crate::AptUpdateLine::MissingSources => AptUpdateLine::MissingSources,
            crate::AptUpdateLine::MalformedSources => AptUpdateLine::MalformedSources,
            crate::AptUpdateLine::UnknownRepository(source) => {
                AptUpdateLine::UnknownRepository(source)
            }
            crate::AptUpdateLine::Update(source) => AptUpdateLine::Update(source),
            crate::AptUpdateLine::Connection(failure) => {
                AptUpdateLine::Connection(failure.into_protocol())
            }
            crate::AptUpdateLine::Service(failure) => {
                AptUpdateLine::Service(failure.into_protocol())
            }
            crate::AptUpdateLine::MissingConfig => AptUpdateLine::MissingConfig,
            crate::AptUpdateLine::InvalidConfig => AptUpdateLine::InvalidConfig,
            crate::AptUpdateLine::MalformedConfig => AptUpdateLine::MalformedConfig,
        }
    }
}
