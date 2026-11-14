//! Grey Hack failure text for `aptclient`.

use greyhack_behaviour::ServiceId;

use super::computer::COMPUTER_GONE;
use super::connection::{connection_text, service_text};
use super::file::file_text;
use crate::protocol::{AptFailure, AptUpdateLine};

/// Formats `AptClientIntrinsics` failures of `intrinsic`.
pub(super) fn apt_text(intrinsic: &str, failure: &AptFailure) -> String {
    match failure {
        AptFailure::NoCache => {
            "/etc/apt/aptcache.bin not found. Launch apt with the update option to refresh the file"
                .to_owned()
        }
        AptFailure::InvalidCache => {
            "/etc/apt/aptcache.bin is not a valid apt cache. Launch apt with the update option to refresh the file"
                .to_owned()
        }
        AptFailure::InvalidAddress(address) => format!("{address} is not a valid IP"),
        AptFailure::ListedRepository(address) => {
            format!("{address} it is already added to sources.txt")
        }
        AptFailure::UnlistedRepository(address) => format!("{address} not found in sources.txt"),
        AptFailure::UnknownRepository(address) => format!("{address} repository not found"),
        AptFailure::NoMatch(term) => format!("{term} not found in any repository"),
        AptFailure::UnknownPackage(package) => format!("{package} not found in the server"),
        AptFailure::MissingPackageFile(package) => {
            format!("{package} does not exist in the server")
        }
        AptFailure::ComputerUnavailable => COMPUTER_GONE.to_owned(),
        AptFailure::MissingFolder(folder) => format!("{folder} does not exist in this filesystem"),
        AptFailure::NoPermission => "permission denied".to_owned(),
        AptFailure::Copy(failure) => file_text("install", failure),
        AptFailure::NoInternet => format!("apt_{intrinsic}: No internet access."),
        AptFailure::EmptyPackageName => "package name can not be empty".to_owned(),
        AptFailure::MalformedCache => {
            "/etc/apt/aptcache.bin content is malformed. Launch apt with the update option to refresh the file"
                .to_owned()
        }
        AptFailure::MissingLocalFile(name) => format!("{name} does not exist in this filesystem"),
        AptFailure::Connection(failure) => connection_text(failure),
        AptFailure::Service(failure) => service_text(failure),
        AptFailure::MissingConfig => format!(
            "{} does not exist in the server",
            ServiceId::Repository.config_path()
        ),
        AptFailure::MissingRepositoryFolder => {
            "apt folder content does not exist in the server".to_owned()
        }
    }
}

/// Formats one output line from `AptClientIntrinsics.update`.
pub(in crate::game) fn apt_update_text(line: &AptUpdateLine) -> String {
    match line {
        AptUpdateLine::Update(source) => format!("Updating: {source}"),
        AptUpdateLine::Connection(failure) => connection_text(failure),
        AptUpdateLine::Service(failure) => service_text(failure),
        AptUpdateLine::MissingConfig => format!(
            "{} does not exist in the server",
            ServiceId::Repository.config_path()
        ),
        AptUpdateLine::MalformedConfig => format!(
            "{} file content is malformed in the server",
            ServiceId::Repository.config_path()
        ),
    }
}
