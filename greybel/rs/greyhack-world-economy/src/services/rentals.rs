//! Rented networks: their price, speed changes, and removal, which follow the player's rental
//! subscriptions.

use std::cmp::Ordering;

use greyhack_behaviour::{
    DEFAULT_NET_SPEED, DeviceKind, RENTAL_ADMIN_PRICE, RentalDeviceTable, SubscriptionKind,
    rental_price, rental_price_without_admin,
};

use crate::services::Bank;
use crate::{NetworkId, Result};

/// Methods of [`World`] for economy rentals.
impl Bank {
    pub async fn set_net_speed(&self, public_ip: &str, net_speed: u8) -> Result<bool> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(false);
        };
        let rental_bank = self
            .subscriptions
            .subscription_bank_of(SubscriptionKind::Rented, public_ip)
            .await?;
        let before = match rental_bank {
            Some(_) => Some((
                self.rental_price(router.network).await?,
                self.rentals.rental_admin(router.network).await?.is_some(),
            )),
            None => None,
        };

        self.networks
            .set_net_speed(router.network, net_speed)
            .await?;

        if let (Some(bank), Some((price, had_admin))) = (rental_bank, before) {
            self.refresh_rented_payments(&bank, router.network, price, had_admin)
                .await?;
        }

        Ok(true)
    }

    pub async fn rental_price(&self, network: NetworkId) -> Result<i64> {
        let mut devices = RentalDeviceTable::default();

        for computer in self.computers.computers_in(network).await? {
            match computer.kind {
                DeviceKind::Router => devices.routers += 1,
                DeviceKind::Hub => devices.hubs += 1,
                DeviceKind::Switch => devices.switches += 1,
                _ if computer.id == self.world.world_mut().player => {}
                _ => devices.computers += 1,
            }
        }

        let net_speed = self
            .networks
            .network(network)
            .await?
            .map_or(DEFAULT_NET_SPEED, |network| network.net_speed);

        Ok(rental_price(
            devices,
            self.rentals.rental_admin(network).await?.is_some(),
            net_speed,
        ))
    }

    pub async fn refresh_rented_payments(
        &self,
        bank: &str,
        network: NetworkId,
        price: i64,
        had_admin: bool,
    ) -> Result<()> {
        if self
            .subscriptions
            .subscription(bank, SubscriptionKind::Rented)
            .await?
            .is_none()
        {
            return Ok(());
        }

        let has_admin = self.rentals.rental_admin(network).await?.is_some();
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

    pub async fn remove_network(&self, public_ip: &str) -> Result<bool> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(false);
        };
        let devices = self.computers.computer_ids_in(router.network).await?;

        if devices.contains(&self.world.player()) {
            return Ok(false);
        }

        if let Some(bank) = self
            .subscriptions
            .subscription_bank_of(SubscriptionKind::Rented, public_ip)
            .await?
        {
            let hired = self.rentals.rental_admin(router.network).await?.is_some();
            let price = rental_price_without_admin(self.rental_price(router.network).await?, hired);

            if !self
                .remove_rental_payments(&bank, public_ip, price, hired)
                .await?
            {
                return Ok(false);
            }
        }

        let computer_ids: Vec<u32> = devices.iter().map(|computer| computer.0).collect();

        self.mine_jobs.cancel_computers(&computer_ids);
        self.scheduler
            .cancel_computers(&self.world.db().connection(), &computer_ids)
            .await?;
        self.world.world_mut().process_ends.extend(devices);
        self.networks.remove_network(router.network).await?;

        Ok(true)
    }
}
