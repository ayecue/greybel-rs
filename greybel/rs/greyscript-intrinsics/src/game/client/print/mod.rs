//! Rendering of typed world output in the game intrinsic's wording.

use greybel_interpreter::{PrintOptions, Stdout};

use super::failure::{apt_update_text, exploit_text, failure_text, lan_text, login_text};
use crate::protocol::{FloodWarning, NetUseFailure, PrintLine};

/// Prints `line` to `output` using the wording of the game method that produced it.
pub(crate) fn print_line(output: &dyn Stdout, line: &PrintLine) {
    let text = match line {
        PrintLine::AptUpdate(line) => apt_update_text(line),
        PrintLine::CaptureHandshake { bssid } => format!(
            "Waiting for beacon frame (BSSID: {bssid})\nSending Authentication Request (Open system) [ACK]\nAuthentication successful\nSending Association Request\nAssociation successful :-)"
        ),
        PrintLine::CaptureProgress { packets, acks } => {
            format!("Read {packets} packets (got {acks} ACKs)")
        }
        PrintLine::CaptureSaveFailure(failure) => failure_text("aireplay", failure)
            .expect("aireplay prints a capture-save failure"),
        PrintLine::KeyRecoveryFailure(failure) => {
            failure_text("aircrack", failure).expect("aircrack prints a key-recovery failure")
        }
        PrintLine::PayloadFailure(failure) => {
            failure_text("payload", failure).expect("payload prints an outside-library failure")
        }
        PrintLine::ExploitLog(log) => exploit_text(log),
        PrintLine::NetUseFailure { failure, address } => net_use_text(failure, address),
        PrintLine::HiddenZonesWarning => "**Warning: One or more memory zones with possible vulnerabilities could not be scanned. Update your Metaxploit library.**".to_owned(),
        PrintLine::FloodStart { address, warning } => start_text(address, warning),
        PrintLine::FloodInterrupted => "remote connection interrupted".to_owned(),
        PrintLine::LoginFailure(failure) => login_text(failure),
        PrintLine::EncryptionEnabled => "Encryption enabled...".to_owned(),
        PrintLine::LaunchFailure(failure) => {
            failure_text("launch", failure).expect("launch prints a launch refusal")
        }
    };

    output.print(&text, PrintOptions::default());
}

/// Formats `MetaExploitIntrinsics.net_use` failure text for `address`.
fn net_use_text(failure: &NetUseFailure, address: &str) -> String {
    match failure {
        NetUseFailure::InvalidAddress => format!("Invalid ip address: {address}"),
        NetUseFailure::UnknownAddress => "Ip address not found.".to_owned(),
        NetUseFailure::MainRouterDown => {
            "net_use: the main router of the network is down.".to_owned()
        }
        NetUseFailure::Lan(failure) => lan_text(*failure).to_owned(),
        NetUseFailure::UnknownComputer => "Error: LAN computer not found.".to_owned(),
        NetUseFailure::MissingPort => "Port not found.".to_owned(),
        NetUseFailure::ClosedPort => "can't connect: port closed.".to_owned(),
        NetUseFailure::InvalidLibrary(library) => format!("Error: Not a valid {library} library."),
        NetUseFailure::UnexpectedLibrary(name) => {
            format!("Unexpected library found. Not a valid {name} library.")
        }
    }
}

/// Text printed when a flood of `address` starts, with the `warning` the game shows.
fn start_text(address: &str, warning: &FloodWarning) -> String {
    let warning = match warning {
        FloodWarning::None => "",
        FloodWarning::SuperAdmin => {
            "\nWarning: Super administrator detected. Additional DDoS traffic required."
        }
        FloodWarning::CrownShield => "\nWarning: DDoS traffic blocked by CrownShield.",
    };

    format!("Flooding {address}...\nPress Ctrl+C to stop{warning}")
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
