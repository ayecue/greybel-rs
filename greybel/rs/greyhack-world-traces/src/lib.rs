#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack traces domain: active and passive administrator traces and player strikes.
//!
//! [`TracesModule`] registers the domain schema, services, routine kinds, and event subscriber.
//! Public entities and services are re-exported at the crate root; repositories remain private to
//! the domain.

pub mod model;
pub mod routines;
pub mod services;
pub mod storage;

use std::rc::Rc;

use greyhack_world_core::{CallFuture, PlayerTraceService, WorldRef};
pub use greyhack_world_core::{ComputerId, PersonId, Result, TraceKind, WorldError};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_routines::Routine;
use greyhack_world_storage::DomainSchema;
pub use model::{PassiveState, TraceId};
pub use routines::{ActiveTrace, PassiveTrace};
pub use services::{StrikeEvent, TraceService};
use storage::TraceRepository;

/// Services and repositories of the traces domain.
#[derive(Clone)]
pub struct TracesModule {
    /// Active and passive traces and player strikes.
    pub traces: Rc<TraceService>,
    /// Active-trace expiry routine.
    pub active: Rc<ActiveTrace>,
    /// Passive-trace stepping routine.
    pub passive: Rc<PassiveTrace>,
}

impl TracesModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let scheduler = registry.scheduler();
        let trace_repository = Rc::new(TraceRepository::new(world.db().clone()));
        let traces = Rc::new(TraceService::new(registry, trace_repository));

        Self {
            traces,
            active: Rc::new(ActiveTrace::new(world, registry.get(), scheduler.clone())),
            passive: Rc::new(PassiveTrace::new(world, registry.get(), scheduler)),
        }
    }

    /// Returns the traces schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [
                storage::schema::GROUP,
                ActiveTrace::tables(),
                PassiveTrace::tables(),
                storage::schema::VOLATILE,
            ],
        )
    }
}

impl Module for TracesModule {
    const NAME: &'static str = "traces";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_service::<dyn PlayerTraceService>(module.traces.clone());
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
        registry.register_service(module.traces.clone());
        registry.register_service(module.active.clone());
        registry.register_service(module.passive.clone());

        module
    }
}

impl PlayerTraceService for TraceService {
    fn player_token_trace<'a>(&'a self, _: &'a WorldRef) -> CallFuture<'a, String> {
        Box::pin(async move { self.player_token_trace().await })
    }
}
