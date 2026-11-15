//! Connecting to a service on a computer: its online check and the login handshake
//! (`PlayerUtils.ConnectToService`, `Networking.CheckServiceOnline`).

use std::rc::Rc;

use greyhack_behaviour::{FileKind, ServiceId, is_lan_ip, is_valid_ip, library_path};
use greyhack_world_modules::Dep;

use super::reach::ReachFailure;
use crate::storage::*;
use crate::systems::{NetworkConnect as NetworkConnectService, *};
use crate::*;

/// Owned login data published to packet-capture subscribers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPacket {
    /// Computer the login comes from.
    pub source: ComputerId,
    /// Computer that was logged in to.
    pub destination: ComputerId,
    /// Address the login connected to.
    pub address: String,
    /// Port the login connected to.
    pub port: u16,
    /// User of the login.
    pub user: String,
    /// Password of the login.
    pub password: String,
}

impl greyhack_world_effects::Event for LoginPacket {}

struct ServiceLogin<'a> {
    from: ComputerId,
    /// Computers running the script terminals.
    terminal_computers: &'a [ComputerId],
    address: &'a str,
    port: u16,
    service: &'a str,
    user: &'a str,
    password: &'a str,
}

/// A service a connection needs online (`Networking.CheckServiceOnline`).
#[derive(Clone, Copy)]
pub struct ServiceCheck<'a> {
    pub target: ComputerId,
    pub service: ServiceId,
    /// Public address of the connecting computer.
    pub source: &'a str,
    pub address: &'a str,
    pub port: u16,
    /// Whether to reject connections a LAN firewall blocks.
    pub firewall: bool,
    /// Computers running the script terminals.
    pub terminal_computers: &'a [ComputerId],
}

/// Service login result after reaching a computer, regardless of credential acceptance
/// (`PlayerUtils.ConnectToService`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHandshake {
    pub target: ComputerId,
    pub result: Result<(), LoginFailure>,
}

impl NetworkConnectService {
    /// Checks whether `service` on `target` is available from public `source` through
    /// `address:port` for terminals on `terminal_computers` (`Networking.CheckServiceOnline`). With
    /// `firewall`, also rejects LAN-blocked connections.
    pub async fn check_service(&self, check: &ServiceCheck<'_>) -> Result<Option<ServiceFailure>> {
        let ServiceCheck {
            target,
            service,
            source,
            address,
            port,
            firewall,
            terminal_computers,
        } = *check;

        if !is_valid_ip(source) {
            return Ok(Some(ServiceFailure::NoInternet));
        }

        if !self.reach.is_available(target).await?
            || !self
                .reach
                .is_network_active(target, terminal_computers)
                .await?
        {
            return Ok(Some(ServiceFailure::HostDown));
        }

        let Some(open_port) = self
            .computer_rows
            .service_port(target, service)
            .await?
            .map(|open| open.number)
        else {
            return Ok(Some(ServiceFailure::MissingService(service)));
        };

        if is_lan_ip(address) {
            if open_port != port {
                return Ok(Some(ServiceFailure::InvalidServicePort));
            }
        } else {
            let forward = match self.topology.main_router_of(target).await? {
                Some(router) => self
                    .router_rows
                    .port_forward(router.id, port)
                    .await?
                    .map(|forward| forward.port.number),
                None => None,
            };

            match forward {
                None => return Ok(Some(ServiceFailure::MissingPortForward)),
                Some(forward) if forward != open_port => {
                    return Ok(Some(ServiceFailure::InvalidServicePort));
                }
                Some(_) => {}
            }
        }

        let Some(file) = self
            .lookup
            .file(target, &library_path(service.library()), true)
            .await?
        else {
            return Ok(Some(ServiceFailure::MissingLibrary(service)));
        };

        if file
            .data()
            .is_none_or(|data| data.kind != FileKind::Library)
        {
            return Ok(Some(ServiceFailure::InvalidLibrary(service)));
        }

        let main_router = self.topology.main_router_of(target).await?;

        if let Some(router) = &main_router
            && !self.reach.is_available(router.id.computer()).await?
        {
            return Ok(Some(ServiceFailure::MainRouterDown));
        }

        if !self.startup.is_valid_library(&file).await? {
            return Ok(Some(ServiceFailure::UnexpectedLibrary(
                file.name().to_owned(),
            )));
        }

        if firewall
            && let Some(router) = main_router
            && let Some(failure) = self
                .reach
                .firewall_ping_status(router.network, source, address, port)
                .await?
        {
            return Ok(Some(ServiceFailure::Lan(failure)));
        }

        // `Computer.IsOutGoingConnectionOk`: the target and every ancestor device must start.
        if !self.reach.is_reachable_through(target).await? {
            return Ok(Some(ServiceFailure::RemoteNetworkDown));
        }

        Ok(None)
    }

    /// Authenticates a remote SSH or FTP login and records its plaintext packet
    /// (`PlayerUtils.ConnectToService`, `PlayerUtils.ConnectService`).
    ///
    /// # Errors
    ///
    /// Returns any database lookup error.
    #[allow(clippy::too_many_arguments)]
    pub async fn connect_service(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
        service: &str,
        user: &str,
        password: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<ComputerId, LoginFailure>> {
        let handshake = self
            .connect_service_handshake(
                from,
                address,
                port,
                service,
                user,
                password,
                terminal_computers,
            )
            .await?;

        if let Ok(handshake) = &handshake {
            self.world.events().publish(LoginPacket {
                source: from,
                destination: handshake.target,
                address: address.to_owned(),
                port,
                user: user.to_owned(),
                password: password.to_owned(),
            });
        }

        Ok(handshake.and_then(|handshake| handshake.result.map(|()| handshake.target)))
    }

    /// Authenticates a service login without recording a packet. Retains reached computers after
    /// startup, firewall, or password failure. Service-online failures return before encryption
    /// (`PlayerUtils.ConnectToService`, `ShellIntrinsics.connect_service`).
    ///
    /// # Errors
    ///
    /// Returns any database lookup error.
    #[allow(clippy::too_many_arguments)]
    pub async fn connect_service_handshake(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
        service: &str,
        user: &str,
        password: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<ServiceHandshake, LoginFailure>> {
        let login = ServiceLogin {
            from,
            terminal_computers,
            address,
            port,
            service,
            user,
            password,
        };

        self.connect_service_impl(login).await
    }

    async fn connect_service_impl(
        &self,
        login: ServiceLogin<'_>,
    ) -> Result<Result<ServiceHandshake, LoginFailure>> {
        let ServiceLogin {
            from,
            terminal_computers,
            address,
            port,
            service,
            user,
            password,
        } = login;
        let Some(service_id) = ServiceId::from_name(service)
            .filter(|id| matches!(id, ServiceId::Ssh | ServiceId::Ftp))
        else {
            return Ok(Err(LoginFailure::InvalidService));
        };
        let target = match self.reach.reach_with_target(from, address, port).await? {
            Ok(target) => target,
            Err(ReachFailure::BeforeComputer(failure)) => {
                return Ok(Err(LoginFailure::Connection(failure)));
            }
            Err(ReachFailure::AtComputer { target, failure }) => {
                return Ok(Ok(ServiceHandshake {
                    target,
                    result: Err(LoginFailure::Connection(failure)),
                }));
            }
        };
        let source = self.topology.public_ip(from).await?;

        if let Some(failure) = self
            .check_service(&ServiceCheck {
                target,
                service: service_id,
                source: &source,
                address,
                port,
                firewall: false,
                terminal_computers,
            })
            .await?
        {
            return Ok(Err(LoginFailure::Service(failure)));
        }

        let result = match self.users.check_login(target, user, password).await? {
            Login::Valid => Ok(()),
            Login::InvalidCredentials => Err(LoginFailure::IncorrectPassword),
            Login::AccountInactive => Err(LoginFailure::AccountInactive),
        };

        Ok(Ok(ServiceHandshake { target, result }))
    }

    /// Resolves a computer through `PlayerUtils.ConnectComputer`, preserving typed failures for
    /// procedures such as repository updates.
    ///
    /// # Errors
    ///
    /// Returns any database lookup error.
    pub async fn connect_computer(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
    ) -> Result<Result<ComputerId, ConnectionFailure>> {
        self.reach.reach(from, address, port).await
    }
}

/// Service and computer connections.
pub struct NetworkConnect {
    pub(crate) world: WorldRef,
    pub(crate) reach: Dep<NetworkReach>,
    pub(crate) topology: Dep<NetworkTopology>,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) startup: Dep<Startup>,
    pub(crate) users: Dep<UserService>,
}

impl NetworkConnect {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            reach: registry.get(),
            topology: registry.get(),
            computer_rows,
            router_rows,
            lookup: registry.get(),
            startup: registry.get(),
            users: registry.get(),
        }
    }
}
