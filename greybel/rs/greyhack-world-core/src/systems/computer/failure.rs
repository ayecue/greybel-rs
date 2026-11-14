//! Why a script's request to a computer is refused (`ComputerIntrinsics`).

use greyhack_behaviour::Refusal;

use crate::{NetCardFailure, UserFailure};

/// Why a computer refusal a script request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputerFailure {
    /// Root privileges are required.
    RootOnly,
    /// Rebooting requires root or membership in the root group.
    NoRebootPermission,
    /// Safe mode can reboot only the player computer.
    RemoteSafeMode,
    /// The process is one the system protects.
    SystemProcess,
    /// The process belongs to the program started by its terminal on the player computer.
    ScriptProcess,
    /// The process belongs to another user.
    ForeignProcess {
        /// Process ID.
        pid: i32,
        /// Owner of the process.
        user: String,
    },
    /// There is no user with the name.
    UnknownUser(String),
    /// The user is not in the group.
    UnknownGroup {
        /// Group name.
        group: String,
        /// User name.
        user: String,
    },
    /// Root cannot be deleted.
    RootDeletion,
    /// The user cannot be deleted.
    UndeletableUser,
    /// The user account is not created.
    User(UserFailure),
    /// Guests cannot change the network.
    GuestUser,
    /// The computer has no network card of the name.
    MissingNetDevice,
    /// `connect_wifi` requires a Wi-Fi card.
    WifiCardsOnly,
    /// `connect_ethernet` requires an Ethernet card.
    EthernetCardsOnly,
    /// The program switched networks less than a second ago.
    ReconnectTooSoon,
    /// A rented server's computer only joins its own network.
    RentedComputer,
    /// The router's network is disabled for not having been paid for.
    DisabledNetwork,
    /// There is no router with the Wi-Fi.
    UnknownWifi {
        /// BSSID of the Wi-Fi.
        bssid: String,
        /// ESSID of the Wi-Fi.
        essid: String,
    },
    /// The router's Wi-Fi is out of the computer's reach.
    OutOfReach,
    /// The Wi-Fi password does not match.
    IncorrectWifiPassword,
    /// The Ethernet address is not an IP address.
    InvalidAddress,
    /// The Ethernet gateway is not an IP address.
    InvalidGateway,
    /// The Ethernet address and gateway are not in one subnet.
    DifferentSubnets,
    /// The network did not take the computer.
    NetCard(NetCardFailure),
    /// An argument breaks a rule.
    Refusal(Refusal),
}
