use greyhack_behaviour::{LanFailure, ServiceId};

/// Service availability errors (`Networking.CheckServiceOnline`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceFailure {
    /// The connection source has no public address.
    NoInternet,
    /// The computer is offline or restarting.
    HostDown,
    /// The computer does not run the service.
    MissingService(ServiceId),
    /// The service port does not match the direct or forwarded connection port.
    InvalidServicePort,
    /// The public port forward no longer exists.
    MissingPortForward,
    /// The computer has no service library file in `/lib`.
    MissingLibrary(ServiceId),
    /// The service library file is not a library.
    InvalidLibrary(ServiceId),
    /// The computer network main router is rebooting.
    MainRouterDown,
    /// The resolved service library does not match its filename; contains the actual library
    /// filename.
    UnexpectedLibrary(String),
    /// The LAN check rejected the connection.
    Lan(LanFailure),
    /// A device between the computer and its main router cannot start
    /// (`Computer.IsOutGoingConnectionOk`).
    RemoteNetworkDown,
}
