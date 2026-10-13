//! Conversions of LAN, connection, service, login, and Metaxploit session failures.

use super::{
    ConnectionFailure, FirewallEntry, LanFailure, LoginFailure, NetUseFailure, RshellFailure,
    ServiceFailure,
};

impl From<greyhack_world::LanFailure> for LanFailure {
    fn from(failure: greyhack_world::LanFailure) -> Self {
        match failure {
            greyhack_world::LanFailure::Unreachable => Self::Unreachable,
            greyhack_world::LanFailure::Firewall => Self::Firewall,
        }
    }
}

impl From<greyhack_world::ConnectionFailure> for ConnectionFailure {
    fn from(failure: greyhack_world::ConnectionFailure) -> Self {
        use greyhack_world::ConnectionFailure as World;

        match failure {
            World::UnknownAddress => Self::UnknownAddress,
            World::RouterUnavailable => Self::RouterUnavailable,
            World::MissingPort(port) => Self::MissingPort(port),
            World::MissingComputerPort(port) => Self::MissingComputerPort(port),
            World::ClosedPort => Self::ClosedPort,
            World::NoMachineBehindPort(port) => Self::NoMachineBehindPort(port),
            World::ServiceNotBehindPort(port) => Self::ServiceNotBehindPort(port),
            World::DisabledNetwork => Self::DisabledNetwork,
            World::Startup(failures) => {
                Self::Startup(failures.into_iter().map(Into::into).collect())
            }
            World::Lan(failure) => Self::Lan(failure.into()),
        }
    }
}

impl From<greyhack_world::ServiceFailure> for ServiceFailure {
    fn from(failure: greyhack_world::ServiceFailure) -> Self {
        use greyhack_world::ServiceFailure as World;

        match failure {
            World::NoInternet => Self::NoInternet,
            World::HostDown => Self::HostDown,
            World::MissingService(service) => Self::MissingService(service.name().into()),
            World::InvalidServicePort => Self::InvalidServicePort,
            World::MissingPortForward => Self::MissingPortForward,
            World::MissingLibrary(service) => Self::MissingLibrary(service.library()),
            World::InvalidLibrary(service) => Self::InvalidLibrary(service.library()),
            World::MainRouterDown => Self::MainRouterDown,
            World::UnexpectedLibrary(name) => Self::UnexpectedLibrary(name),
            World::Lan(failure) => Self::Lan(failure.into()),
            World::RemoteNetworkDown => Self::RemoteNetworkDown,
        }
    }
}

impl From<greyhack_world::LoginFailure> for LoginFailure {
    fn from(failure: greyhack_world::LoginFailure) -> Self {
        use greyhack_world::LoginFailure as World;

        match failure {
            World::InvalidService => Self::InvalidService,
            World::Connection(failure) => Self::Connection(failure.into()),
            World::Service(failure) => Self::Service(failure.into()),
            World::IncorrectPassword => Self::IncorrectPassword,
            World::AccountInactive => Self::AccountInactive,
        }
    }
}

impl From<greyhack_world::NetUseFailure> for NetUseFailure {
    fn from(failure: greyhack_world::NetUseFailure) -> Self {
        use greyhack_world::NetUseFailure as World;

        match failure {
            World::InvalidAddress => Self::InvalidAddress,
            World::UnknownAddress => Self::UnknownAddress,
            World::MainRouterDown => Self::MainRouterDown,
            World::Lan(failure) => Self::Lan(failure.into()),
            World::UnknownComputer => Self::UnknownComputer,
            World::MissingPort => Self::MissingPort,
            World::ClosedPort => Self::ClosedPort,
            World::InvalidLibrary(service) => Self::InvalidLibrary(service.library()),
            World::UnexpectedLibrary(name) => Self::UnexpectedLibrary(name),
        }
    }
}

impl RshellFailure {
    /// Converts a world connection `failure` for `address:port`.
    pub(crate) fn of(failure: greyhack_world::RshellFailure, address: &str, port: u16) -> Self {
        use greyhack_world::RshellFailure as World;

        match failure {
            World::UnknownAddress => Self::UnknownAddress(address.to_owned()),
            World::MissingServer => Self::MissingServer(port),
            World::Service(failure) => Self::Service(failure.into()),
        }
    }
}

impl From<greyhack_behaviour::FirewallRule> for FirewallEntry {
    fn from(rule: greyhack_behaviour::FirewallRule) -> Self {
        Self {
            allow: rule.allow,
            port: rule.port,
            source: rule.source,
            destination: rule.destination,
        }
    }
}
