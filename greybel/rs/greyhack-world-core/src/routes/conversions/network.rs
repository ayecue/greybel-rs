//! Conversions of LAN, connection, service, login, and firewall values.

use greyhack_world_programs::protocol::{
    ConnectionFailure, FirewallEntry, LanFailure, LoginFailure, ServiceFailure,
};

use super::IntoProtocol;

impl IntoProtocol for greyhack_behaviour::LanFailure {
    type Output = LanFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            greyhack_behaviour::LanFailure::Unreachable => LanFailure::Unreachable,
            greyhack_behaviour::LanFailure::Firewall => LanFailure::Firewall,
        }
    }
}

impl IntoProtocol for crate::ConnectionFailure {
    type Output = ConnectionFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::ConnectionFailure::UnknownAddress => ConnectionFailure::UnknownAddress,
            crate::ConnectionFailure::RouterUnavailable => ConnectionFailure::RouterUnavailable,
            crate::ConnectionFailure::MissingPort(port) => ConnectionFailure::MissingPort(port),
            crate::ConnectionFailure::MissingComputerPort(port) => {
                ConnectionFailure::MissingComputerPort(port)
            }
            crate::ConnectionFailure::ClosedPort => ConnectionFailure::ClosedPort,
            crate::ConnectionFailure::NoMachineBehindPort(port) => {
                ConnectionFailure::NoMachineBehindPort(port)
            }
            crate::ConnectionFailure::ServiceNotBehindPort(port) => {
                ConnectionFailure::ServiceNotBehindPort(port)
            }
            crate::ConnectionFailure::DisabledNetwork => ConnectionFailure::DisabledNetwork,
            crate::ConnectionFailure::Startup(failures) => ConnectionFailure::Startup(
                failures
                    .into_iter()
                    .map(IntoProtocol::into_protocol)
                    .collect(),
            ),
            crate::ConnectionFailure::Lan(failure) => {
                ConnectionFailure::Lan(failure.into_protocol())
            }
        }
    }
}

impl IntoProtocol for crate::ServiceFailure {
    type Output = ServiceFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::ServiceFailure::NoInternet => ServiceFailure::NoInternet,
            crate::ServiceFailure::HostDown => ServiceFailure::HostDown,
            crate::ServiceFailure::MissingService(service) => {
                ServiceFailure::MissingService(service.name().into())
            }
            crate::ServiceFailure::InvalidServicePort => ServiceFailure::InvalidServicePort,
            crate::ServiceFailure::MissingPortForward => ServiceFailure::MissingPortForward,
            crate::ServiceFailure::MissingLibrary(service) => {
                ServiceFailure::MissingLibrary(service.library())
            }
            crate::ServiceFailure::InvalidLibrary(service) => {
                ServiceFailure::InvalidLibrary(service.library())
            }
            crate::ServiceFailure::MainRouterDown => ServiceFailure::MainRouterDown,
            crate::ServiceFailure::UnexpectedLibrary(name) => {
                ServiceFailure::UnexpectedLibrary(name)
            }
            crate::ServiceFailure::Lan(failure) => ServiceFailure::Lan(failure.into_protocol()),
            crate::ServiceFailure::RemoteNetworkDown => ServiceFailure::RemoteNetworkDown,
        }
    }
}

impl IntoProtocol for crate::LoginFailure {
    type Output = LoginFailure;

    fn into_protocol(self) -> Self::Output {
        let failure = self;

        match failure {
            crate::LoginFailure::InvalidService => LoginFailure::InvalidService,
            crate::LoginFailure::Connection(failure) => {
                LoginFailure::Connection(failure.into_protocol())
            }
            crate::LoginFailure::Service(failure) => LoginFailure::Service(failure.into_protocol()),
            crate::LoginFailure::IncorrectPassword => LoginFailure::IncorrectPassword,
            crate::LoginFailure::AccountInactive => LoginFailure::AccountInactive,
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
