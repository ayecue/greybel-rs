//! Administrators of rented servers (`NetworkHelperServer`, `BankHelperServer`).

use greyhack_behaviour::{NetworkKind, RENTAL_ADMIN_PRICE, SubscriptionKind, kind_for_ip};

use crate::generation::NetworkOptions;
use crate::{NetworkId, Npc, PersonId, PurchaseFailure, Result, Rng, World};

/// Rented-server administrator hiring and firing errors
/// (`BankHelperServer.HireAdminRentalServerRpc`, `FireAdminRentalServerRpc`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RentalAdminFailure {
    /// The network administrator is alive and not fired.
    AdminExists,
    /// The network has no hired administrator.
    NoAdmin,
    /// The player cannot pay the administrator's first charge.
    Purchase(PurchaseFailure),
    /// The player has no monthly admin charge registered, which removing an administrator takes a charge off.
    NoAdminPayment,
}

impl World {
    /// Hires an administrator for network `public_ip`
    /// (`BankHelperServer.HireAdminRentalServerRpc`). An administrator who died or was fired is
    /// removed first, together with their monthly charge (`NetworkLan.RemoveInvalidSuperAdminRental`,
    /// `BankSubs.RemoveAdminPayment`). The player then pays the administrator's first month from
    /// their bank account and the monthly charge is registered (`BankSubs.AddPayment`). Returns
    /// `None` without a network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn hire_rental_admin(
        &mut self,
        public_ip: &str,
    ) -> Result<Option<Result<PersonId, RentalAdminFailure>>> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(None);
        };
        let bank = self.player_bank().await?;

        if self.db.rental_admin(router.network).await?.is_some()
            && self.rental_admin(router.network).await?.is_none()
        {
            self.db.set_rental_admin(router.network, None).await?;

            if let Some(bank) = &bank {
                self.remove_admin_payment(bank).await?;
            }
        }

        if self.db.rental_admin(router.network).await?.is_some() {
            return Ok(Some(Err(RentalAdminFailure::AdminExists)));
        }

        if let Err(failure) = self
            .pay_purchase(RENTAL_ADMIN_PRICE, "Hire admin services", false)
            .await?
        {
            return Ok(Some(Err(RentalAdminFailure::Purchase(failure))));
        }

        if let Some(bank) = &bank {
            self.add_subscription(bank, SubscriptionKind::Admin, RENTAL_ADMIN_PRICE, "")
                .await?;
        }

        Ok(Some(Ok(self
            .spawn_rental_admin(public_ip, router.network)
            .await?)))
    }

    /// Makes an administrator for network `public_ip` from a person of a new common-kind network
    /// (`ServerMap.SpawnRouter`, `NetworkLan.SetSuperAdminRental`), who intercepts all attacks while
    /// alive and employed (`NetworkLan.IsSuperAdminRental`).
    pub(in crate::systems) async fn spawn_rental_admin(
        &mut self,
        public_ip: &str,
        network: NetworkId,
    ) -> Result<PersonId> {
        let mut rng = Rng::stable(
            self.seed,
            &[public_ip, "rental admin", &self.time_seconds.to_string()],
        );
        // Create a common-kind network (`ServerMap.SpawnRouter`). Equal address-space allocation
        // per kind makes rejection sampling select kinds uniformly. Network people become NPCs.
        let candidates = loop {
            let address = rng.public_ip();

            if !NetworkKind::COMMON.contains(&kind_for_ip(&self.world_seed, &address))
                || self.router_at(&address).await?.is_some()
            {
                continue;
            }

            let employer = self
                .add_network(&NetworkOptions {
                    public_ip: Some(address),
                    ..NetworkOptions::default()
                })
                .await?;
            let Some(employer) = self.db.router(employer).await? else {
                continue;
            };
            let candidates: Vec<PersonId> = self
                .db
                .npcs_in(employer.network)
                .await?
                .into_iter()
                .map(|npc| npc.person)
                .collect();

            if !candidates.is_empty() {
                break candidates;
            }
        };
        let admin = *rng.pick(&candidates);

        self.db.set_rental_admin(network, Some(admin)).await?;

        Ok(admin)
    }

    /// Removes the hired administrator from network `public_ip`
    /// (`BankHelperServer.FireAdminRentalServerRpc`) or reports none, taking their charge off the
    /// player's admin subscription (`BankSubs.RemoveAdminPayment`), which must exist. Returns `None`
    /// without a network.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn fire_rental_admin(
        &mut self,
        public_ip: &str,
    ) -> Result<Option<Result<(), RentalAdminFailure>>> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(None);
        };

        if self.db.rental_admin(router.network).await?.is_none() {
            return Ok(Some(Err(RentalAdminFailure::NoAdmin)));
        }

        let has_payment = match self.player_bank().await? {
            Some(bank) => self.remove_admin_payment(&bank).await?,
            None => false,
        };

        if !has_payment {
            return Ok(Some(Err(RentalAdminFailure::NoAdminPayment)));
        }

        self.db.set_rental_admin(router.network, None).await?;

        Ok(Some(Ok(())))
    }

    /// Returns `network`'s hired administrator while alive and not fired
    /// (`NetworkLan.IsSuperAdminRental`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn rental_admin(&self, network: NetworkId) -> Result<Option<Npc>> {
        let Some(person) = self.db.rental_admin(network).await? else {
            return Ok(None);
        };

        Ok(self
            .db
            .npc(person)
            .await?
            .filter(|npc| !npc.is_dead_or_fired(self.time_seconds)))
    }

    /// Returns whether a super administrator intercepts `network` attacks. Rented servers use their
    /// hired administrator; other networks require a qualified, alive, employed administrator
    /// (`MetaLibIntrinsics.overflow`, `NpcInfo.IsSuperAdmin`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn has_super_admin(&self, network: NetworkId) -> Result<bool> {
        let Some(row) = self.db.network(network).await? else {
            return Ok(false);
        };

        if row.kind == NetworkKind::RentServer {
            return Ok(self.rental_admin(network).await?.is_some());
        }

        let Some(admin) = self.db.network_admin(network).await? else {
            return Ok(false);
        };
        let gone = match self.db.network_admin_person(network).await? {
            Some(person) => self
                .db
                .npc(person)
                .await?
                .is_some_and(|npc| npc.is_dead_or_fired(self.time_seconds)),
            None => false,
        };

        Ok(admin.is_super_admin() && !gone)
    }
}

#[cfg(test)]
#[path = "tests/rental_admin.rs"]
mod tests;
