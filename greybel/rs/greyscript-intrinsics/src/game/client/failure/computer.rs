//! Grey Hack failure text for computer users, processes, and network cards.

use greyhack_world_assets::GameTextTable;

use super::connection::UNPAID_REMOTE;
use crate::protocol::{ComputerFailure, NetCardFailure, ProcessFailure, UserFailure};

/// Failure text for an unavailable computer.
pub(in crate::game) const COMPUTER_GONE: &str = "Unknown error: Unable to access to local computer";

/// Formats `failure` for `intrinsic` using `ComputerIntrinsics` messages.
pub(in crate::game) fn computer_text(intrinsic: &str, failure: &ComputerFailure) -> String {
    match failure {
        ComputerFailure::RootOnly => "Denied. Only root user can execute this command.".to_owned(),
        ComputerFailure::NoRebootPermission => {
            "Permission denied. Only root or users in the root group can execute this command."
                .to_owned()
        }
        ComputerFailure::RemoteSafeMode => {
            "Error: unable to restart in safe mode on remote computers.".to_owned()
        }
        ComputerFailure::SystemProcess => "Permission denied. Process protected.".to_owned(),
        ComputerFailure::ScriptProcess => {
            "Unable to close this process. The process belongs to this script.".to_owned()
        }
        ComputerFailure::ForeignProcess { pid, user } => {
            format!("Permission denied. PID {pid} belongs to user <b>{user}</b>")
        }
        ComputerFailure::UnknownUser(user) => match intrinsic {
            "change_password" => format!("user {user} does not exist"),
            "delete_user" => format!("can't delete user. {user} does not exist"),
            _ => format!("Error: user {user} does not exist."),
        },
        ComputerFailure::UnknownGroup { group, user } => {
            format!("Error: group {group} not found in user {user}")
        }
        ComputerFailure::RootDeletion => "the root user can't be deleted.".to_owned(),
        ComputerFailure::UndeletableUser => {
            "sorry, at the moment that account can't be deleted.".to_owned()
        }
        ComputerFailure::User(failure) => user_text(failure),
        ComputerFailure::Process(failure) => process_text(*failure),
        ComputerFailure::GuestUser => {
            format!("{intrinsic}: permission denied. Guest users can not execute this method")
        }
        ComputerFailure::MissingNetDevice => format!("{intrinsic}: Network device not found"),
        ComputerFailure::WifiCardsOnly => "connect_wifi: Only wifi cards are supported".to_owned(),
        ComputerFailure::EthernetCardsOnly => {
            "connect_ethernet: Only ethernet cards are supported".to_owned()
        }
        ComputerFailure::RentedComputer => {
            "connect_wifi: rented computers cannot connect to another network.".to_owned()
        }
        ComputerFailure::DisabledNetwork => UNPAID_REMOTE.to_owned(),
        ComputerFailure::ReconnectTooSoon => {
            format!("{intrinsic}: You must wait a moment before reconnecting to another network.")
        }
        ComputerFailure::UnknownWifi { bssid, essid } => {
            format!("Can't connect. Router not found. Bssid: {bssid}. Essid: {essid}")
        }
        ComputerFailure::OutOfReach => "Can't connect. Target is out of reach.".to_owned(),
        ComputerFailure::IncorrectWifiPassword => "Can't connect. Incorrect password.".to_owned(),
        ComputerFailure::InvalidAddress => "Error: Invalid IP address".to_owned(),
        ComputerFailure::InvalidGateway => "Error: invalid gateway".to_owned(),
        ComputerFailure::DifferentSubnets => {
            "Error: the IP address and the gateway must belong to the same subnet".to_owned()
        }
        ComputerFailure::NetCard(failure) => net_card_text(failure),
    }
}

/// Formats `ComputerIntrinsics.create_user` failures.
fn user_text(failure: &UserFailure) -> String {
    match failure {
        UserFailure::RootExists => "Error: root user already exists.".to_owned(),
        UserFailure::ReservedGuest => "Error: can't create guest user. Reserved user.".to_owned(),
        UserFailure::UserExists(user) => {
            format!("Error: can't create user. {user} already exists.")
        }
        UserFailure::TooManyUsers => {
            "Denied. Maximum number of registered users reached.".to_owned()
        }
    }
}

/// Formats `Computer.CheckMemoryAvailable` and terminal-limit failures.
pub(in crate::game) fn process_text(failure: ProcessFailure) -> String {
    match failure {
        ProcessFailure::ComputerUnavailable => COMPUTER_GONE.to_owned(),
        ProcessFailure::TooManyTerminals => {
            "Maximum number of Terminals in execution reached.".to_owned()
        }
        ProcessFailure::NoRam { remote } => GameTextTable::english()
            .get(if remote {
                "NO_RAM_REMOTE"
            } else {
                "NO_RAM_MSG"
            })
            .unwrap_or("Not enough RAM memory.")
            .to_owned(),
    }
}

/// Formats `NetworkLan.ConnectWifi` and `NetworkLan.ConnectEthernet` failures.
fn net_card_text(failure: &NetCardFailure) -> String {
    match failure {
        NetCardFailure::ComputerUnavailable => COMPUTER_GONE.to_owned(),
        NetCardFailure::UnknownRouter => "router not found".to_owned(),
        NetCardFailure::DisabledHomeNetwork => {
            "can't connect: the home network has been temporarily disabled due to non-payment"
                .to_owned()
        }
        NetCardFailure::SelfGateway => {
            "Error: the gateway cannot be the same device to connect".to_owned()
        }
        NetCardFailure::UnknownGateway(gateway) => {
            format!("Error: gateway {gateway} does not exist in the network")
        }
        NetCardFailure::AddressInUse(address) => {
            format!("Error: address {address} is already in use.")
        }
        NetCardFailure::NoWifiCard => "Error: no wifi card found".to_owned(),
        NetCardFailure::NoEthernetCard => "Error: no ethernet card found".to_owned(),
    }
}
