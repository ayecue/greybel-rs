//! Network records and settings: disabling, connection speed, and kind.

use std::rc::Rc;

use greyhack_behaviour::NetworkKind;

use crate::storage::{ComputerRepository, RouterRepository};
use crate::systems::NetworkSettings as NetworkSettingsService;
use crate::{ComputerId, Network, NetworkId, Result, RouterId};

impl NetworkSettingsService {
    /// Returns network `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn network(&self, id: NetworkId) -> Result<Option<Network>> {
        self.router_rows.network(id).await
    }

    /// Returns whether the network containing `computer`'s main router is disabled for nonpayment
    /// (`NetworkLan.disabledNetwork`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_disabled(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.computer_rows.computer(computer).await? else {
            return Ok(false);
        };

        Ok(self
            .router_rows
            .network(row.network)
            .await?
            .is_some_and(|network| network.disabled))
    }

    /// Enables or disables the network at `public_ip` for nonpayment (`BankSubs.DisableNetwork`).
    /// Returns false if no network exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn disable_network(&self, public_ip: &str, disabled: bool) -> Result<bool> {
        let Some(router) = self.router_rows.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.router_rows
            .disable_network(router.network, disabled)
            .await?;

        Ok(true)
    }

    /// Sets the main router's home marker (`Router.ResetHomeNetwork` when cleared).
    pub async fn set_home_network(&self, public_ip: &str, home: bool) -> Result<bool> {
        let Some(router) = self.router_rows.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.router_rows
            .set_home_network(router.network, home)
            .await?;

        Ok(true)
    }

    /// Returns whether the network containing `computer` is its main router's home network.
    pub async fn is_home_network(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.computer_rows.computer(computer).await? else {
            return Ok(false);
        };

        Ok(self
            .router_rows
            .network(row.network)
            .await?
            .is_some_and(|network| network.home))
    }

    /// Returns connection speed for `router`'s network (`NetworkLan.netSpeed`), or `None` if the
    /// router is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn net_speed(&self, router: RouterId) -> Result<Option<u8>> {
        let Some(row) = self.router_rows.router(router).await? else {
            return Ok(None);
        };

        Ok(self
            .router_rows
            .network(row.network)
            .await?
            .map(|network| network.net_speed))
    }

    /// Marks `id` as a network of `kind` with computers generated for that kind.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn set_network_kind(&self, id: NetworkId, kind: NetworkKind) -> Result<()> {
        self.router_rows.set_network_kind(id, kind).await
    }
}

/// Network settings and availability.
pub struct NetworkSettings {
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
}

impl NetworkSettings {
    pub(crate) fn new(
        computer_rows: Rc<ComputerRepository>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            computer_rows,
            router_rows,
        }
    }
}
