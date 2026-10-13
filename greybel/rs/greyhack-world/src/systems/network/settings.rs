//! Network records and settings: disabling, connection speed, kind, the rental price, and removal.

use std::cmp::Ordering;

use greyhack_behaviour::{
    DEFAULT_NET_SPEED, DeviceKind, NetworkKind, RENTAL_ADMIN_PRICE, RentalDeviceTable,
    SubscriptionKind, rental_price, rental_price_without_admin,
};

use crate::model::{ComputerId, NetworkId, RouterId};
use crate::{Network, Result, World};

impl World {
    /// Returns network `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn network(&self, id: NetworkId) -> Result<Option<Network>> {
        self.db.network(id).await
    }

    /// Returns whether the network containing `computer`'s main router is disabled for nonpayment
    /// (`NetworkLan.disabledNetwork`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_disabled(&self, computer: ComputerId) -> Result<bool> {
        let Some(row) = self.db.computer(computer).await? else {
            return Ok(false);
        };

        Ok(self
            .db
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
    pub async fn disable_network(&mut self, public_ip: &str, disabled: bool) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };

        self.db.disable_network(router.network, disabled).await?;

        Ok(true)
    }

    /// Sets connection speed for the network at `public_ip`, as an ISP package change does
    /// (`NetworkLan.netSpeed`). When the network is one the player rents, the player's rental
    /// charge follows its new price (`NetworkHelperServer.RefreshRentedPayments`). Returns false if
    /// no network exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_net_speed(&mut self, public_ip: &str, net_speed: u8) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };
        let rental_bank = self
            .db
            .subscription_bank_of(SubscriptionKind::Rented, public_ip)
            .await?;
        let before = match rental_bank {
            Some(_) => Some((
                self.rental_price(router.network).await?,
                self.db.rental_admin(router.network).await?.is_some(),
            )),
            None => None,
        };

        self.db.set_net_speed(router.network, net_speed).await?;

        if let (Some(bank), Some((price, had_admin))) = (rental_bank, before) {
            self.refresh_rented_payments(&bank, router.network, price, had_admin)
                .await?;
        }

        Ok(true)
    }

    /// Returns the monthly price of the rented network `network` (`ConfigLanUI.CalculaPrecio` with no
    /// pending changes): its computers except the player's, its routers, hubs and switches, a hired
    /// administrator and its speed pack.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn rental_price(&self, network: NetworkId) -> Result<i64> {
        let mut devices = RentalDeviceTable::default();

        for computer in self.db.computers_in(network).await? {
            match computer.kind {
                DeviceKind::Router => devices.routers += 1,
                DeviceKind::Hub => devices.hubs += 1,
                DeviceKind::Switch => devices.switches += 1,
                _ if computer.id == self.player => {}
                _ => devices.computers += 1,
            }
        }

        let net_speed = self
            .db
            .network(network)
            .await?
            .map_or(DEFAULT_NET_SPEED, |network| network.net_speed);

        Ok(rental_price(
            devices,
            self.db.rental_admin(network).await?.is_some(),
            net_speed,
        ))
    }

    /// Makes the rental and admin subscriptions of `bank` follow a rented network's new price
    /// (`NetworkHelperServer.RefreshRentedPayments`): `price` and `had_admin` are the network's
    /// before the change. Changes nothing without a rental subscription.
    pub(in crate::systems) async fn refresh_rented_payments(
        &mut self,
        bank: &str,
        network: NetworkId,
        price: i64,
        had_admin: bool,
    ) -> Result<()> {
        if self
            .db
            .subscription(bank, SubscriptionKind::Rented)
            .await?
            .is_none()
        {
            return Ok(());
        }

        let has_admin = self.db.rental_admin(network).await?.is_some();
        let admin_change = match (has_admin, had_admin) {
            (true, false) => RENTAL_ADMIN_PRICE,
            (false, true) => -RENTAL_ADMIN_PRICE,
            _ => 0,
        };
        let next = self.rental_price(network).await? - price - admin_change;

        self.add_subscription(bank, SubscriptionKind::Rented, next, "")
            .await?;

        match admin_change.cmp(&0) {
            Ordering::Greater => {
                self.add_subscription(bank, SubscriptionKind::Admin, admin_change, "")
                    .await?;
            }
            Ordering::Less => {
                self.remove_admin_payment(bank).await?;
            }
            Ordering::Equal => {}
        }

        Ok(())
    }

    /// Returns connection speed for `router`'s network (`NetworkLan.netSpeed`), or `None` if the
    /// router is absent.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn net_speed(&self, router: RouterId) -> Result<Option<u8>> {
        let Some(row) = self.db.router(router).await? else {
            return Ok(None);
        };

        Ok(self
            .db
            .network(row.network)
            .await?
            .map(|network| network.net_speed))
    }

    /// Marks `id` as a network of `kind` with computers generated for that kind.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn set_network_kind(&mut self, id: NetworkId, kind: NetworkKind) -> Result<()> {
        self.db.set_network_kind(id, kind).await
    }

    /// Deletes the non-player network at `public_ip`, its computers, and dependent state. When the
    /// network is one the player rents, its charges come off the player's subscriptions
    /// (`NetworkHelperServer.DeleteNetworkServerRpc`, `BankSubs.RemoveRentalPayments`). Returns false
    /// when the network is the player's or a subscription the removal needs is missing, which the
    /// game fails on before removing the network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn remove_network(&mut self, public_ip: &str) -> Result<bool> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(false);
        };
        let devices = self.db.computer_ids_in(router.network).await?;

        if devices.contains(&self.player) {
            return Ok(false);
        }

        if let Some(bank) = self
            .db
            .subscription_bank_of(SubscriptionKind::Rented, public_ip)
            .await?
        {
            let hired = self.db.rental_admin(router.network).await?.is_some();
            let price = rental_price_without_admin(self.rental_price(router.network).await?, hired);

            if !self
                .remove_rental_payments(&bank, public_ip, price, hired)
                .await?
            {
                return Ok(false);
            }
        }

        self.cancel_jobs_on(&devices).await?;
        self.cancel_routines_on(&devices).await?;
        self.process_ends.extend(devices);
        self.db.remove_network(router.network).await?;

        Ok(true)
    }
}
