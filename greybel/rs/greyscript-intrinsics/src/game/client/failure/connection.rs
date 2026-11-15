//! Shared Grey Hack text for world-reported connection failures.
//! `net_use` and `rshell_client` format their intrinsic-specific failures locally.

use greyhack_world_assets::GameTextTable;

use crate::protocol::{
    ConnectionFailure, LanFailure, LoginFailure, ServiceFailure, SmtpFailure, StartupFailure,
};

/// Explains connection refusal for an unpaid, disabled network (`PlayerUtils.ConnectComputer`).
pub(in crate::game) const UNPAID_REMOTE: &str =
    "can't connect: the remote server has been temporarily disabled due to non-payment";

/// Formats `PlayerUtils.FirewallPingStatus` failures.
pub(in crate::game) const fn lan_text(failure: LanFailure) -> &'static str {
    match failure {
        LanFailure::Unreachable => "Connection refused. Address unreachable",
        LanFailure::Firewall => "Connection refused. The target is behind a firewall.",
    }
}

/// Formats `PlayerUtils.ConnectComputer` failures.
pub(in crate::game) fn connection_text(failure: &ConnectionFailure) -> String {
    match failure {
        ConnectionFailure::UnknownAddress => "ip address not found".to_owned(),
        ConnectionFailure::RouterUnavailable => "router is down".to_owned(),
        ConnectionFailure::MissingPort(port) => format!("can't connect: port {port} not found"),
        ConnectionFailure::MissingComputerPort(port) => {
            format!("can't connect: port {port} not found.")
        }
        ConnectionFailure::ClosedPort => "can't connect: port closed".to_owned(),
        ConnectionFailure::NoMachineBehindPort(port) => {
            format!("can't connect: There is no active machine behind the port {port}")
        }
        ConnectionFailure::ServiceNotBehindPort(port) => {
            format!("can't connect: service not found behind the port {port}")
        }
        ConnectionFailure::DisabledNetwork => UNPAID_REMOTE.to_owned(),
        ConnectionFailure::Startup(failures) => startup_text(failures),
        ConnectionFailure::Lan(failure) => lan_text(*failure).to_owned(),
    }
}

/// Formats `Computer.CheckInicio` failures as one line per startup file preventing boot.
pub(in crate::game) fn startup_text(failures: &[StartupFailure]) -> String {
    failures
        .iter()
        .map(|failure| match failure {
            StartupFailure::MissingKernelFile(path) => {
                format!("Kernel panic: missing {path} file.\n")
            }
            StartupFailure::MissingGraphicsFile(path) => {
                format!("Graphics error: {path} not found!\n")
            }
            StartupFailure::MissingSystemFile(path) => format!("Fatal error: {path} not found!\n"),
            StartupFailure::InvalidFile(path) => format!("Fatal error: {path} corrupted!\n"),
        })
        .collect()
}

/// Formats `Networking.CheckServiceOnline` failures.
pub(in crate::game) fn service_text(failure: &ServiceFailure) -> String {
    match failure {
        ServiceFailure::NoInternet => "No internet access.".to_owned(),
        ServiceFailure::HostDown => "Remote host is down".to_owned(),
        ServiceFailure::MissingService(service) => format!("Unable to find service {service}"),
        ServiceFailure::InvalidServicePort => {
            "Invalid target service port configuration.".to_owned()
        }
        ServiceFailure::MissingPortForward => {
            "connection rejected: port forward removed by admin".to_owned()
        }
        ServiceFailure::MissingLibrary(library) => format!("Unable to connect: missing {library}"),
        ServiceFailure::InvalidLibrary(library) => format!("Unable to connect: invalid {library}"),
        ServiceFailure::MainRouterDown => {
            "Unable to connect: the main router of the network is down.".to_owned()
        }
        ServiceFailure::UnexpectedLibrary(name) => {
            format!("Unexpected library found. Not a valid {name} library.")
        }
        ServiceFailure::Lan(failure) => lan_text(*failure).to_owned(),
        ServiceFailure::RemoteNetworkDown => {
            "Unable to connect: The target is not accessible due to a remote network connection error."
                .to_owned()
        }
    }
}

/// Formats `PlayerUtils.ConnectToService` failures.
pub(in crate::game) fn login_text(failure: &LoginFailure) -> String {
    match failure {
        LoginFailure::InvalidService => "Invalid service ID".to_owned(),
        LoginFailure::Connection(failure) => connection_text(failure),
        LoginFailure::Service(failure) => service_text(failure),
        LoginFailure::IncorrectPassword => "can't connect: incorrect user/password".to_owned(),
        LoginFailure::AccountInactive => inactive_account(),
    }
}

/// Formats `PlayerUtils.GetSmtpServer` failures.
pub(in crate::game) fn smtp_text(failure: SmtpFailure) -> String {
    match failure {
        SmtpFailure::InvalidAddress => "invalid IP address".to_owned(),
        SmtpFailure::HostDown => "host is down".to_owned(),
        SmtpFailure::UnknownHost => "host doesn't exist".to_owned(),
        SmtpFailure::UnknownAddress => "ip address not found".to_owned(),
        SmtpFailure::MissingPort(port) => format!("port {port} not found"),
        SmtpFailure::InvalidService => "invalid target service".to_owned(),
        SmtpFailure::MissingService => "service not found".to_owned(),
    }
}

/// Returns Grey Hack's `ACCOUNT_DISABLED` text when the account's NPC is dead or fired.
pub(in crate::game) fn inactive_account() -> String {
    GameTextTable::english()
        .get("ACCOUNT_DISABLED")
        .unwrap_or("ACCOUNT_DISABLED")
        .to_owned()
}
