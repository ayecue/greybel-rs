//! Administrators of rented servers (`NetworkHelperServer`, `BankHelperServer`).

use greyhack_behaviour::{NetworkKind, RENTAL_ADMIN_PRICE, Rng, SubscriptionKind, kind_for_ip};
use greyhack_world_core::{NetworkId, PersonId, Result};
use greyhack_world_economy::PurchaseFailure;

use crate::{JobService, Npc};

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

/// Methods of [`World`] for npcs rental admin.
impl JobService {
    pub async fn hire_rental_admin(
        &self,
        public_ip: &str,
    ) -> Result<Option<Result<PersonId, RentalAdminFailure>>> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };
        let bank = self.bank.player_bank().await?;

        if self.bank.rental_admin(router.network).await?.is_some()
            && self.rental_admin(router.network).await?.is_none()
        {
            self.bank.set_rental_admin(router.network, None).await?;

            if let Some(bank) = &bank {
                self.bank.remove_admin_payment(bank).await?;
            }
        }

        if self.bank.rental_admin(router.network).await?.is_some() {
            return Ok(Some(Err(RentalAdminFailure::AdminExists)));
        }

        if let Err(failure) = self
            .bank
            .pay_purchase(RENTAL_ADMIN_PRICE, "Hire admin services", false)
            .await?
        {
            return Ok(Some(Err(RentalAdminFailure::Purchase(failure))));
        }

        if let Some(bank) = &bank {
            self.bank
                .add_subscription(bank, SubscriptionKind::Admin, RENTAL_ADMIN_PRICE, "")
                .await?;
        }

        Ok(Some(Ok(self
            .spawn_rental_admin(public_ip, router.network)
            .await?)))
    }

    pub async fn spawn_rental_admin(
        &self,
        public_ip: &str,
        network: NetworkId,
    ) -> Result<PersonId> {
        let mut rng = Rng::stable(
            self.world.seed(),
            &[
                public_ip,
                "rental admin",
                &self.world.time_seconds().to_string(),
            ],
        );
        // Create a common-kind network (`ServerMap.SpawnRouter`). Equal address-space allocation
        // per kind makes rejection sampling select kinds uniformly. Network people become NPCs.
        let candidates = loop {
            let address = rng.public_ip();

            if !NetworkKind::COMMON.contains(&kind_for_ip(&self.world.world_seed(), &address))
                || self.networks.routers.router_at(&address).await?.is_some()
            {
                continue;
            }

            let employer = self.network_creation.add_network(&address).await?;
            let Some(employer) = self.networks.router(employer).await? else {
                continue;
            };
            let candidates: Vec<PersonId> = self
                .npcs
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

        self.bank.set_rental_admin(network, Some(admin)).await?;

        Ok(admin)
    }

    pub async fn fire_rental_admin(
        &self,
        public_ip: &str,
    ) -> Result<Option<Result<(), RentalAdminFailure>>> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };

        if self.bank.rental_admin(router.network).await?.is_none() {
            return Ok(Some(Err(RentalAdminFailure::NoAdmin)));
        }

        let has_payment = match self.bank.player_bank().await? {
            Some(bank) => self.bank.remove_admin_payment(&bank).await?,
            None => false,
        };

        if !has_payment {
            return Ok(Some(Err(RentalAdminFailure::NoAdminPayment)));
        }

        self.bank.set_rental_admin(router.network, None).await?;

        Ok(Some(Ok(())))
    }

    pub async fn rental_admin(&self, network: NetworkId) -> Result<Option<Npc>> {
        let Some(person) = self.bank.rental_admin(network).await? else {
            return Ok(None);
        };

        Ok(self
            .npcs
            .npc(person)
            .await?
            .filter(|npc| !npc.is_dead_or_fired(self.world.time_seconds())))
    }

    pub async fn has_super_admin(&self, network: NetworkId) -> Result<bool> {
        let Some(row) = self.networks.network(network).await? else {
            return Ok(false);
        };

        if row.kind == NetworkKind::RentServer {
            return Ok(self.rental_admin(network).await?.is_some());
        }

        let Some(admin) = self.npcs.network_admin(network).await? else {
            return Ok(false);
        };
        let is_unavailable = match self.npcs.network_admin_person(network).await? {
            Some(person) => self
                .npcs
                .npc(person)
                .await?
                .is_some_and(|npc| npc.is_dead_or_fired(self.world.time_seconds())),
            None => false,
        };

        Ok(admin.is_super_admin() && !is_unavailable)
    }
}
