#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack guilds domain: guild records, company ownership and share prices, the Crown
//! leaderboard, CrownShield protection, and active and passive administrator traces.
//!
//! [`GuildModule`] registers the domain schema, services, routine kinds, and event subscribers.
//! The composition root registers [`routes::register`] for its concrete world. [`StrikeEvent`]
//! lets the player domain react to a trace strike and [`FloodClear`] lets intrusion clear floods
//! when CrownShield is hired. Public entities and services are re-exported at the crate root;
//! repositories remain private to the domain.

pub mod model;
pub mod routes;
pub mod routines;
pub mod services;
pub mod storage;

use std::rc::Rc;

pub use greyhack_behaviour::{Rng, Word};
pub use greyhack_world_core::{ComputerId, NetworkId, PersonId, Result, TraceKind, WorldError};
use greyhack_world_core::{HookFuture, PlayerTraces, WorldRef};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_router::WorldContext;
use greyhack_world_routines::Routine;
use greyhack_world_storage::DomainSchema;
pub use model::{
    AdminRestoreFailure, CompanyLayout, CrownRank, CrownShieldFailure, CrownState, Guild,
    GuildAbsence, GuildDraft, GuildFailure, GuildId, GuildLayout, GuildsLayout, PassiveState,
    Stake, TraceId,
};
pub use routines::{ActiveTrace, PassiveTrace};
pub use services::{FloodClear, GuildService, StrikeEvent, Traces};
use storage::{GuildRepository, TraceRepository};

/// Services and repositories of the guild domain.
#[derive(Clone)]
pub struct GuildModule {
    /// Guild records, companies, the Crown leaderboard, and CrownShield.
    pub guilds: Rc<GuildService>,
    /// Active and passive traces and player strikes.
    pub traces: Rc<Traces>,
    /// Active-trace expiry routine.
    pub active: Rc<ActiveTrace>,
    /// Passive-trace stepping routine.
    pub passive: Rc<PassiveTrace>,
}

impl GuildModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let scheduler = registry.scheduler();
        let db = world.db().clone();
        let guild_repository = Rc::new(GuildRepository::new(db.clone()));
        let trace_repository = Rc::new(TraceRepository::new(db));
        let guilds = Rc::new(GuildService::new(
            world,
            guild_repository,
            registry.get(),
            registry.get(),
            registry.get(),
        ));
        let traces = Rc::new(Traces::new(registry, trace_repository));

        Self {
            guilds: guilds.clone(),
            traces: traces.clone(),
            active: Rc::new(ActiveTrace::new(world, registry.get(), scheduler.clone())),
            passive: Rc::new(PassiveTrace::new(world, registry.get(), scheduler)),
        }
    }

    /// Returns the guilds schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [
                storage::schema::GROUP,
                storage::schema::TRACE_GROUP,
                ActiveTrace::tables(),
                PassiveTrace::tables(),
                storage::schema::VOLATILE,
            ],
        )
    }
}

impl Module for GuildModule {
    const NAME: &'static str = "guilds";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_service::<dyn PlayerTraces>(module.traces.clone());
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
        registry.register_async_subscriber({
            let traces = module.traces.clone();

            move |event: &greyhack_world_core::TraceAction| {
                let traces = traces.clone();

                Box::pin(async move {
                    traces
                        .trace_action(event.remote, event.bounces, event.action, event.terminal)
                        .await
                        .map(|_| ())
                        .map_err(greyhack_world_modules::EventError::new)
                })
            }
        });
        registry.register_routine(module.active.clone());
        registry.register_routine(module.passive.clone());
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.guilds.clone());
        registry.register_service(module.traces.clone());
        registry.register_service(module.active.clone());
        registry.register_service(module.passive.clone());

        module
    }
}

impl PlayerTraces for Traces {
    fn player_token_trace<'a>(&'a self, _: &'a WorldRef) -> HookFuture<'a, String> {
        Box::pin(async move { self.player_token_trace().await })
    }
}

/// Access to guild, NPC, economy, and core services from a concrete assembled world.
pub trait GuildContext:
    greyhack_world_npcs::NpcContext + WorldContext<Computer = ComputerId>
{
    /// Returns this world's guild module.
    fn guilds(&self) -> Rc<GuildModule>;
}
