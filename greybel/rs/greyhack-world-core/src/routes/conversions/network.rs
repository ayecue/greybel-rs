//! Conversions of LAN, connection, service, login, and firewall values.

use super::{
    ConnectionFailure, FirewallEntry, IntoProtocol, LanFailure, LoginFailure, ServiceFailure,
};

impl IntoProtocol for crate::LanFailure {
    type Output = LanFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::LanFailure::Unreachable => LanFailure::Unreachable,
            crate::LanFailure::Firewall => LanFailure::Firewall,
        }
    }
}

impl IntoProtocol for crate::ConnectionFailure {
    type Output = ConnectionFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ConnectionFailure as World;

        match failure {
            World::UnknownAddress => ConnectionFailure::UnknownAddress,
            World::RouterUnavailable => ConnectionFailure::RouterUnavailable,
            World::MissingPort(port) => ConnectionFailure::MissingPort(port),
            World::MissingComputerPort(port) => ConnectionFailure::MissingComputerPort(port),
            World::ClosedPort => ConnectionFailure::ClosedPort,
            World::NoMachineBehindPort(port) => ConnectionFailure::NoMachineBehindPort(port),
            World::ServiceNotBehindPort(port) => ConnectionFailure::ServiceNotBehindPort(port),
            World::DisabledNetwork => ConnectionFailure::DisabledNetwork,
            World::Startup(failures) => ConnectionFailure::Startup(
                failures
                    .into_iter()
                    .map(IntoProtocol::into_protocol)
                    .collect(),
            ),
            World::Lan(failure) => ConnectionFailure::Lan(failure.into_protocol()),
        }
    }
}

impl IntoProtocol for crate::ServiceFailure {
    type Output = ServiceFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::ServiceFailure as World;

        match failure {
            World::NoInternet => ServiceFailure::NoInternet,
            World::HostDown => ServiceFailure::HostDown,
            World::MissingService(service) => ServiceFailure::MissingService(service.name().into()),
            World::InvalidServicePort => ServiceFailure::InvalidServicePort,
            World::MissingPortForward => ServiceFailure::MissingPortForward,
            World::MissingLibrary(service) => ServiceFailure::MissingLibrary(service.library()),
            World::InvalidLibrary(service) => ServiceFailure::InvalidLibrary(service.library()),
            World::MainRouterDown => ServiceFailure::MainRouterDown,
            World::UnexpectedLibrary(name) => ServiceFailure::UnexpectedLibrary(name),
            World::Lan(failure) => ServiceFailure::Lan(failure.into_protocol()),
            World::RemoteNetworkDown => ServiceFailure::RemoteNetworkDown,
        }
    }
}

impl IntoProtocol for crate::LoginFailure {
    type Output = LoginFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        use crate::LoginFailure as World;

        match failure {
            World::InvalidService => LoginFailure::InvalidService,
            World::Connection(failure) => LoginFailure::Connection(failure.into_protocol()),
            World::Service(failure) => LoginFailure::Service(failure.into_protocol()),
            World::IncorrectPassword => LoginFailure::IncorrectPassword,
            World::AccountInactive => LoginFailure::AccountInactive,
        }
    }
}

impl IntoProtocol for greyhack_behaviour::FirewallRule {
    type Output = FirewallEntry;

    fn into_protocol(self) -> Self::Output {
        let rule = self;

        FirewallEntry {
            allow: rule.allow,
            port: rule.port,
            source: rule.source,
            destination: rule.destination,
        }
    }
}
