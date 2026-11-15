use greyhack_behaviour::{LanFailure, ServiceId};

/// `metaxploit.net_use` session errors (`MetaExploitIntrinsics.net_use`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetUseFailure {
    /// The address is not a valid IP address.
    InvalidAddress,
    /// The public address is not assigned to a network.
    UnknownAddress,
    /// The network main router is rebooting.
    MainRouterDown,
    /// LAN rules reject the connection; blocking firewalls reject only the service port.
    Lan(LanFailure),
    /// No computer has the LAN address or occupies the forward destination.
    UnknownComputer,
    /// The computer, or router for a public address, has no matching port.
    MissingPort,
    /// The router port is closed.
    ClosedPort,
    /// The service library file in `/lib` is not a library.
    InvalidLibrary(ServiceId),
    /// The resolved service library does not match its filename; contains the actual library
    /// filename.
    UnexpectedLibrary(String),
}
