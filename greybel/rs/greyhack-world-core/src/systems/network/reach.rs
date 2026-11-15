//! Network reachability: routing a connection to a computer through routers, LANs, and firewalls.

use std::rc::Rc;

use greyhack_behaviour::{DeviceKind, NO_ADDRESS, is_device, is_lan_ip, is_valid_ip};
use greyhack_world_modules::Dep;

use super::lan::TablePort;
use crate::storage::*;
use crate::systems::{NetworkReach as NetworkReachService, *};
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReachFailure {
    BeforeComputer(ConnectionFailure),
    AtComputer {
        target: ComputerId,
        failure: ConnectionFailure,
    },
}

impl ReachFailure {
    fn into_failure(self) -> ConnectionFailure {
        match self {
            Self::BeforeComputer(failure) | Self::AtComputer { failure, .. } => failure,
        }
    }
}

impl From<ConnectionFailure> for ReachFailure {
    fn from(failure: ConnectionFailure) -> Self {
        Self::BeforeComputer(failure)
    }
}

impl NetworkReachService {
    /// Returns whether `computer` has network access for a script whose terminals run on
    /// `terminal_computers` (`Computer.IsNetworkActive`, `HelperScript.pendingShell`). Requires a
    /// public address, network card and gateway except for routers/switches, availability of the
    /// computer and main router, and availability of terminal computers and their connected devices
    /// except `computer` and its main router. Terminals run on the first terminal's connected
    /// computer (`MainClass.Main`) and shell-started terminals (`ShellIntrinsics.start_terminal`),
    /// not program-launch targets.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_network_active(
        &self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
    ) -> Result<bool> {
        let Some(row) = self.computer_rows.computer(computer).await? else {
            return Ok(false);
        };

        if self.topology.public_ip(computer).await? == NO_ADDRESS {
            return Ok(false);
        }

        if !is_device(row.kind) {
            let connected = self
                .hardware_rows
                .net_cards(computer)
                .await?
                .iter()
                .any(|card| card.active);

            if !connected || row.gateway.is_empty() || row.gateway == NO_ADDRESS {
                return Ok(false);
            }
        }

        if !self.is_available(computer).await? {
            return Ok(false);
        }

        let main = self
            .topology
            .main_router(row.network)
            .await?
            .map(|router| router.id.computer());

        if let Some(main) = main
            && main != computer
            && !self.is_available(main).await?
        {
            return Ok(false);
        }

        for &shell in terminal_computers {
            if shell != computer && Some(shell) != main && !self.is_reachable_through(shell).await?
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Returns whether `computer` and non-hub ancestors are available (`NetworkLan.ReversePing`).
    pub async fn is_reachable_through(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.computer_rows.computer(computer).await? else {
            return Ok(false);
        };

        if !self.is_available(computer).await? {
            return Ok(false);
        }

        let Some(lan) = self.lan.lan(row.network).await? else {
            return Ok(true);
        };
        let mut node = lan
            .nodes()
            .find(|(_, node)| node.key == computer)
            .map(|(id, _)| id);

        while let Some(id) = node {
            let entry = lan.node(id);

            if entry.kind != DeviceKind::Hub && !self.is_available(entry.key).await? {
                return Ok(false);
            }

            node = lan.parent(id);
        }

        Ok(true)
    }

    /// Checks for open `port` on `computer`.
    pub async fn listens(&self, computer: ComputerId, port: u16) -> Result<bool> {
        Ok(self.computer_rows.port(computer, port).await?.is_some())
    }

    /// Returns whether `computer` is available: not rebooting and all startup checks pass
    /// (`Computer.IsAvailable`).
    pub async fn is_available(&self, computer: ComputerId) -> Result<bool> {
        let rebooting = self
            .computer_rows
            .computer(computer)
            .await?
            .is_none_or(|computer| computer.is_rebooting(self.world.time_seconds()));

        Ok(!rebooting && self.startup.startup_failures(computer).await?.is_empty())
    }

    /// Resolves `address:port` from `from` (`PlayerUtils.ConnectComputer`) or returns the
    /// connection failure.
    pub async fn reach(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
    ) -> Result<Result<ComputerId, ConnectionFailure>> {
        Ok(self
            .reach_with_target(from, address, port)
            .await?
            .map_err(ReachFailure::into_failure))
    }

    /// Runs `PlayerUtils.ConnectComputer` and retains a reached computer after startup or firewall
    /// failure.
    pub async fn reach_with_target(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
    ) -> Result<Result<ComputerId, ReachFailure>> {
        // The shell public address is the connection source (`connect_service` passes
        // `shell.publicIp`).
        let source = self.topology.public_ip(from).await?;

        if is_valid_ip(address) && is_lan_ip(address) {
            return self.reach_lan(from, &source, address, port).await;
        }

        let Some(router) = self
            .network_generation
            .router_or_generate(&self.world, address)
            .await?
        else {
            return Ok(Err(ConnectionFailure::UnknownAddress.into()));
        };

        if self.settings.is_disabled(router.computer()).await? {
            return Ok(Err(ConnectionFailure::DisabledNetwork.into()));
        }

        let target = match self.reach_through(router, port, false).await? {
            Ok(target) => target,
            Err(failure) => return Ok(Err(failure)),
        };
        let Some(router) = self.router_rows.router(router).await? else {
            return Ok(Err(ConnectionFailure::UnknownAddress.into()));
        };

        Ok(
            match self
                .firewall_ping_status(router.network, &source, address, port)
                .await?
            {
                Some(failure) => Err(ReachFailure::AtComputer {
                    target,
                    failure: ConnectionFailure::Lan(failure),
                }),
                None => Ok(target),
            },
        )
    }

    /// Implements `PlayerUtils.ConnectComputerLan` and LAN firewall handling for `ConnectComputer`,
    /// using `address` in the network at public `source`.
    async fn reach_lan(
        &self,
        from: ComputerId,
        source: &str,
        address: &str,
        port: u16,
    ) -> Result<Result<ComputerId, ReachFailure>> {
        let Some(router) = self.router_rows.router_at(source).await? else {
            return Ok(Err(ConnectionFailure::RouterUnavailable.into()));
        };
        let Some(lan) = self.lan.lan(router.network).await? else {
            return Ok(Err(ConnectionFailure::RouterUnavailable.into()));
        };
        let Some(node) = lan.find(address).filter(|node| !lan.node(*node).hidden) else {
            return Ok(Err(ConnectionFailure::UnknownAddress.into()));
        };
        let node = lan.node(node);
        let target = if matches!(node.kind, DeviceKind::Router | DeviceKind::Switch) {
            match self
                .reach_through(RouterId::of(node.key), port, true)
                .await?
            {
                Ok(target) => target,
                Err(failure) => return Ok(Err(failure)),
            }
        } else if self.listens(node.key, port).await? {
            node.key
        } else {
            return Ok(Err(ConnectionFailure::MissingComputerPort(port).into()));
        };
        let origin = if is_lan_ip(source) {
            source.to_owned()
        } else {
            self.computer_rows
                .computer(from)
                .await?
                .map(|computer| computer.local_ip)
                .unwrap_or_default()
        };

        Ok(match lan.firewall_ping_status(&origin, address, port) {
            Some(failure) => Err(ReachFailure::AtComputer {
                target,
                failure: ConnectionFailure::Lan(failure),
            }),
            None => Ok(target),
        })
    }

    /// Resolves the computer behind `router` public `number` through its port table (WAN
    /// `ConnectComputer`). With `lan` (`forceLan`), closed ports are also accepted.
    async fn reach_through(
        &self,
        router: RouterId,
        number: u16,
        lan: bool,
    ) -> Result<Result<ComputerId, ReachFailure>> {
        let table = self.lan.port_table(router).await?;
        let Some(port) = table.iter().find(|port| port.number == number) else {
            return Ok(Err(ConnectionFailure::MissingPort(number).into()));
        };

        if !lan && is_closed(&table, number) {
            return Ok(Err(ConnectionFailure::ClosedPort.into()));
        }

        let Some(router_row) = self.router_rows.router(router).await? else {
            return Ok(Err(ConnectionFailure::UnknownAddress.into()));
        };
        let computer = self.lan.lan(router_row.network).await?.and_then(|lan| {
            lan.find(&port.lan_ip)
                .filter(|node| !lan.node(*node).hidden)
                .map(|node| lan.node(node).key)
        });
        let Some(computer) = computer else {
            return Ok(Err(ConnectionFailure::NoMachineBehindPort(number).into()));
        };

        if !self.listens(computer, port.port.number).await? {
            return Ok(Err(ConnectionFailure::ServiceNotBehindPort(number).into()));
        }

        let failures = self.startup.startup_failures(computer).await?;

        Ok(if failures.is_empty() {
            Ok(computer)
        } else {
            Err(ReachFailure::AtComputer {
                target: computer,
                failure: ConnectionFailure::Startup(failures),
            })
        })
    }

    /// Runs `PlayerUtils.FirewallPingStatus` within `network`.
    pub async fn firewall_ping_status(
        &self,
        network: NetworkId,
        source: &str,
        destination: &str,
        port: u16,
    ) -> Result<Option<greyhack_behaviour::LanFailure>> {
        Ok(match self.lan.lan(network).await? {
            Some(lan) => lan.firewall_ping_status(source, destination, port),
            None => None,
        })
    }

    /// Returns whether `address` responds to `from` (`shell.ping`). LAN targets must be reachable
    /// and online (`NetworkLan.Ping`); public targets require a network and available router.
    ///
    /// # Errors
    ///
    /// Returns any database lookup error.
    pub async fn ping(&self, from: ComputerId, address: &str) -> Result<bool> {
        let Some(computer) = self.computer_rows.computer(from).await? else {
            return Ok(false);
        };
        let Some(lan) = self.lan.lan(computer.network).await? else {
            return Ok(false);
        };

        if !lan.ping(&computer.local_ip, address) {
            return Ok(false);
        }

        if is_lan_ip(address) {
            return match lan.find(address) {
                Some(node) => self.is_available(lan.node(node).key).await,
                None => Ok(false),
            };
        }

        match self
            .network_generation
            .router_or_generate(&self.world, address)
            .await?
        {
            Some(router) => self.is_available(router.computer()).await,
            None => Ok(false),
        }
    }
}

/// Returns whether `table` port `number` is closed (`Puertos.IsClosed`), comparing against external
/// port numbers.
pub fn is_closed(table: &[TablePort], number: u16) -> bool {
    table
        .iter()
        .find(|entry| entry.number == number)
        .is_some_and(|entry| entry.port.closed)
}

/// Network reachability and firewall probing.
pub struct NetworkReach {
    pub(crate) world: WorldRef,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
    pub(crate) lan: Dep<NetworkLan>,
    pub(crate) topology: Dep<NetworkTopology>,
    pub network_generation: Dep<dyn crate::NetworkGeneration>,
    pub(crate) hardware_rows: Rc<HardwareRepository>,
    pub(crate) startup: Dep<Startup>,
    pub(crate) settings: Dep<NetworkSettings>,
}

impl NetworkReach {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        router_rows: Rc<RouterRepository>,
        hardware_rows: Rc<HardwareRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            network_generation: registry.get(),
            computer_rows,
            router_rows,
            lan: registry.get(),
            topology: registry.get(),
            hardware_rows,
            startup: registry.get(),
            settings: registry.get(),
        }
    }
}
