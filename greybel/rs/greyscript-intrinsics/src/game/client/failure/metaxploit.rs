//! Grey Hack failure text for `metaxploit.so` and `rshell_client`.

use super::connection::service_text;
use crate::protocol::{
    MetaxploitFailure, RshellFailure, RshellRemoveFailure, WeaknessInfo, WeaknessRequirement,
};

/// Formats `MetaExploitIntrinsics.rshell_server` failures.
pub(super) const fn metaxploit_text(failure: MetaxploitFailure) -> &'static str {
    match failure {
        MetaxploitFailure::RshellNotRunning => "error: service rshelld is not running",
        MetaxploitFailure::RshellForwardMissing => {
            "error: rshell portforward is not configured correctly"
        }
    }
}

/// Formats `MetaExploitIntrinsics.rshell_client` failures.
pub(super) fn rshell_text(failure: &RshellFailure) -> String {
    match failure {
        RshellFailure::UnknownAddress(address) => {
            format!("rshell_client: IP address not found: {address}")
        }
        RshellFailure::MissingServer(port) => {
            format!("rshell_client: unable to find remote server at port {port}")
        }
        RshellFailure::Service(failure) => service_text(failure),
        RshellFailure::Limit => {
            "rshell_client: connection refused. The server has reached its limit of 16 connections."
                .to_owned()
        }
    }
}

/// Formats `MetaExploitIntrinsics.rshell_remove` failures.
pub(super) const fn rshell_remove_text(failure: RshellRemoveFailure) -> &'static str {
    match failure {
        RshellRemoveFailure::NoService => "error: service rshelld is not running",
        RshellRemoveFailure::ForwardMissing => {
            "error: rshell portforward is not configured correctly"
        }
        RshellRemoveFailure::CtfNetwork => {
            "rshell_remove: The service cannot be started on this network."
        }
        RshellRemoveFailure::ExpectedList => {
            "rshell_remove: expected a list of reverse shells or null"
        }
        RshellRemoveFailure::OnlyReverseShells => {
            "rshell_remove: list must contain only reverse shells"
        }
    }
}

/// Formats a weakness with its requirements (`Vulnerabilidad.PrintInfo`).
pub(in crate::game) fn weakness_text(weakness: &WeaknessInfo) -> String {
    let mut text = format!(
        "{} <b>{}</b>. Buffer overflow.\n",
        weakness.details, weakness.unsec_value
    );

    for requirement in &weakness.requirements {
        text.push_str(&requirement_text(requirement));
    }

    text
}

/// Formats the requirement line of a weakness (`Vulnerabilidad.PrintInfo`).
fn requirement_text(requirement: &WeaknessRequirement) -> String {
    match requirement {
        WeaknessRequirement::Library { name, version } => {
            format!("* Using namespace <b>{name}</b> compiled at version >= <b>{version}</b>\n")
        }
        WeaknessRequirement::RegisteredUsers(count) => {
            format!("* Checking registered users equal to {count}.\n")
        }
        WeaknessRequirement::AnyActiveUser => "* Checking an active user.\n".to_owned(),
        WeaknessRequirement::RootActiveUser => "* Checking root active user.\n".to_owned(),
        WeaknessRequirement::GuestActiveUser => "* Checking guest active user.\n".to_owned(),
        WeaknessRequirement::PortForward(count) => {
            format!("* {count} port forwarding configured from router to the target computer.\n")
        }
        WeaknessRequirement::ConnGateway(count) => {
            format!("* {count} computers using this router as gateway.\n")
        }
        WeaknessRequirement::LocalNetwork => {
            "* Checking existing connection in the local network.\n".to_owned()
        }
        WeaknessRequirement::PathExist(path) => {
            format!("* Checking path {path} exists in the file system.\n")
        }
    }
}
