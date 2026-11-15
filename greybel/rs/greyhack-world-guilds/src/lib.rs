#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack guilds domain: guild records, company ownership and share prices, the Crown
//! leaderboard, and CrownShield protection.
//!
//! [`GuildModule`] registers the domain schema, services, and event subscribers. The composition
//! root registers [`routes::register`] for its concrete world. [`FloodClear`] lets intrusion clear
//! floods when CrownShield is hired. Public entities and services are re-exported at the crate
//! root; repositories remain private to the domain.

pub mod model;
pub mod routes;
pub mod services;
pub mod storage;

use std::rc::Rc;

pub use greyhack_behaviour::{Rng, Word};
use greyhack_world_core::WorldRef;
pub use greyhack_world_core::{ComputerId, NetworkId, PersonId, Result, WorldError};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use model::{
    AdminRestoreFailure, CompanyLayout, CrownRank, CrownShieldFailure, CrownState, Guild,
    GuildAbsence, GuildDraft, GuildFailure, GuildId, GuildLayout, GuildsLayout, Stake,
};
pub use services::{FloodClear, GuildService};
use storage::GuildRepository;

/// Services and repositories of the guild domain.
#[derive(Clone)]
pub struct GuildModule {
    /// Guild records, companies, the Crown leaderboard, and CrownShield.
    pub guilds: Rc<GuildService>,
}

impl GuildModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let db = world.db().clone();
        let guild_repository = Rc::new(GuildRepository::new(db));
        let guilds = Rc::new(GuildService::new(
            world,
            guild_repository,
            registry.get(),
            registry.get(),
            registry.get(),
        ));

        Self { guilds }
    }

    /// Returns the guilds schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(<Self as Module>::NAME, [storage::schema::GROUP])
    }
}

impl Module for GuildModule {
    const NAME: &'static str = "guilds";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_async_subscriber({
            let guilds = module.guilds.clone();

            move |_: &greyhack_world_core::ClockAdvance| {
                let guilds = guilds.clone();

                Box::pin(async move {
                    guilds
                        .update_crown_leaderboard()
                        .await
                        .map_err(greyhack_world_modules::EventError::new)
                })
            }
        });
        registry.register_async_subscriber({
            let guilds = module.guilds.clone();

            move |_: &greyhack_world_core::ClockAdvanceEnd| {
                let guilds = guilds.clone();

                Box::pin(async move {
                    guilds
                        .update_crown_leaderboard()
                        .await
                        .map_err(greyhack_world_modules::EventError::new)
                })
            }
        });
        registry.register_async_subscriber({
            let guilds = module.guilds.clone();

            move |event: &greyhack_world_npcs::AdminArrival| {
                let guilds = guilds.clone();

                Box::pin(async move {
                    guilds
                        .mark_company_admin_arrival(event.network)
                        .await
                        .map_err(greyhack_world_modules::EventError::new)
                })
            }
        });
        registry.register_async_subscriber({
            let guilds = module.guilds.clone();

            move |event: &greyhack_world_npcs::AdminAbsence| {
                let guilds = guilds.clone();

                Box::pin(async move {
                    guilds
                        .apply_admin_absence_penalty(event.network)
                        .await
                        .map_err(greyhack_world_modules::EventError::new)
                })
            }
        });
        registry.register_async_subscriber({
            let guilds = module.guilds.clone();

            move |event: &greyhack_world_npcs::NpcJobChange| {
                let guilds = guilds.clone();

                Box::pin(async move {
                    guilds
                        .sync_company_market(event.network)
                        .await
                        .map_err(greyhack_world_modules::EventError::new)
                })
            }
        });
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.guilds.clone());

        module
    }
}
