//! Lookups along the network topology: computers, their devices, main routers, and public addresses.

use std::rc::Rc;

use greyhack_behaviour::{NO_ADDRESS, is_device};
use greyhack_world_modules::Dep;

use crate::storage::{ComputerRepository, RouterRepository};
use crate::systems::{NetworkLan, NetworkTopology as NetworkTopologyService};
use crate::{Computer, ComputerId, NetworkId, Result, Router, RouterId};

impl NetworkTopologyService {
    /// Returns computer `id`, or `None` if removed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn computer(&self, id: ComputerId) -> Result<Option<Computer>> {
        self.computer_rows.computer(id).await
    }

    /// Updates the `computer` row.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_computer(&self, computer: &Computer) -> Result<()> {
        self.computer_rows.save_computer(computer).await
    }

    /// Returns computers connected through `router` in join order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn devices(&self, router: RouterId) -> Result<Vec<ComputerId>> {
        self.computer_rows.computer_ids_of(router).await
    }

    /// Returns the LAN device at `lan_ip` in `network` (`NetworkLan.FindDevice`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn device(&self, network: NetworkId, lan_ip: &str) -> Result<Option<ComputerId>> {
        Ok(self
            .lan
            .lan(network)
            .await?
            .and_then(|lan| lan.find(lan_ip).map(|id| lan.node(id).key)))
    }

    /// Returns the main router of `network`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn main_router(&self, network: NetworkId) -> Result<Option<Router>> {
        Ok(self
            .router_rows
            .routers_of(network)
            .await?
            .into_iter()
            .find(Router::is_main))
    }

    /// Returns the public address for `computer`: its network address for routers, switches, and
    /// connected computers, or [`NO_ADDRESS`] outside a network.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn public_ip(&self, computer: ComputerId) -> Result<String> {
        let router = match self.computer_rows.computer(computer).await? {
            Some(row) if row.router.is_none() && is_device(row.kind) => {
                self.router_rows.router(RouterId::of(computer)).await?
            }
            Some(_) => self.router_of(computer).await?,
            None => None,
        };

        Ok(router.map_or_else(|| NO_ADDRESS.to_owned(), |router| router.public_ip))
    }

    /// Returns the router through which `computer` connects (`Computer.GetRouter`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn router_of(&self, computer: ComputerId) -> Result<Option<Router>> {
        let Some(router) = self
            .computer_rows
            .computer(computer)
            .await?
            .and_then(|computer| computer.router)
        else {
            return Ok(None);
        };

        self.router_rows.router(router).await
    }

    /// Returns the main router for `computer` (`Computer.GetMainRouter`).
    pub async fn main_router_of(&self, computer: ComputerId) -> Result<Option<Router>> {
        match self.computer_rows.computer(computer).await? {
            Some(computer) => self.main_router(computer.network).await,
            None => Ok(None),
        }
    }

    /// Returns the public address of `network`'s main router.
    pub async fn network_public_ip(&self, network: NetworkId) -> Result<String> {
        Ok(self
            .main_router(network)
            .await?
            .map(|router| router.public_ip)
            .unwrap_or_default())
    }
}

/// Computer and network topology.
pub struct NetworkTopology {
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
    pub(crate) lan: Dep<NetworkLan>,
}

impl NetworkTopology {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            computer_rows,
            router_rows,
            lan: registry.get(),
        }
    }
}
