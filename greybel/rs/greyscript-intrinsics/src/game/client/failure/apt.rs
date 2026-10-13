//! Grey Hack failure text for `aptclient`.

use greyhack_behaviour::ServiceId;

use super::computer::COMPUTER_GONE;
use super::connection::{connection_text, service_text};
use super::file::file_text;
use crate::protocol::{AptFailure, AptUpdateLine};

/// Formats `AptClientIntrinsics` failures.
pub(super) fn apt_text(failure: &AptFailure) -> String {
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
