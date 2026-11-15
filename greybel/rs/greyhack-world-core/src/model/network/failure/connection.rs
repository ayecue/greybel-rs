use greyhack_behaviour::LanFailure;

use crate::StartupFailure;

/// Connection errors from `PlayerUtils.ConnectComputer` and `ConnectComputerLan`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionFailure {
    /// The public address has no network, or the LAN has no reachable computer at the LAN address.
    UnknownAddress,
    /// The source public address has no router.
    RouterUnavailable,
    /// The router port table has no entry with this number.
    MissingPort(u16),
    /// The LAN computer does not listen on this port.
    MissingComputerPort(u16),
    /// The router port is closed.
    ClosedPort,
    /// No reachable computer exists at the forward destination.
    NoMachineBehindPort(u16),
    /// The forward destination does not listen on the internal port.
    ServiceNotBehindPort(u16),
    /// The network is disabled for nonpayment (`NetworkLan.disabledNetwork`).
    DisabledNetwork,
    /// The forward destination cannot start; contains startup failures in check order.
    Startup(Vec<StartupFailure>),
    /// The computer was reached, but LAN rules rejected the connection.
    Lan(LanFailure),
}
