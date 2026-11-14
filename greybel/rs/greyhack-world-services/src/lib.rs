#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack services domain: apt sources and packages, repository caches and upgrades,
//! computer-service installation and control, shop catalogs, and SQL-service records.
//!
//! [`ServicesModule`] registers the domain schema, services, and service-restart routine. The
//! composition root registers [`routes::register`] for its concrete world. Library files reference
//! stored library identities through core. Public entities, object handles, and services are
//! re-exported at the crate root; repositories remain private.

pub mod model;
pub mod objects;
pub mod routes;
pub mod services;
pub mod storage;

use std::rc::Rc;

use greyhack_world_core::WorldRef;
pub use greyhack_world_core::{
    ComputerId, ConnectionFailure, FileFailure, LibraryId, Node, NodeDraft, Port, Result,
    ServiceFailure, WorldError,
};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_routines::Routine;
use greyhack_world_storage::DomainSchema;
pub use model::*;
pub use objects::{AptClientHandle, ServiceHandle, ServiceRef};
pub use services::*;
use storage::AptRepository;

/// Services and the package-repository table of this domain.
#[derive(Clone)]
pub struct ServicesModule {
    /// Package sources, repositories, installation, and upgrades.
    pub apt: Rc<Apt>,
    /// Root-only computer-service installation, start, and stop.
    pub control: Rc<ServiceControl>,
    /// Service files, ports, shop catalogs, and records.
    pub installs: Rc<Installs>,
    /// Service-restart routine.
    pub restart: Rc<ServiceRestart>,
}

impl ServicesModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let scheduler = registry.scheduler();
        let repositories = Rc::new(AptRepository::new(world.db().clone()));
        let installs = Rc::new(Installs::new(
            registry.get(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));
        let apt = Rc::new(Apt::new(
            repositories,
            registry.get(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));
        let control = Rc::new(ServiceControl::new(
            registry.get(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));

        Self {
            apt,
            control,
            installs,
            restart: Rc::new(ServiceRestart::new(world, registry.get(), scheduler)),
        }
    }

    /// Returns the services schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [storage::schema::software::GROUP, ServiceRestart::tables()],
        )
    }
}

impl Module for ServicesModule {
    const NAME: &'static str = "services";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_routine(module.restart.clone());
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.apt.clone());
        registry.register_service(module.control.clone());
        registry.register_service(module.installs.clone());
        registry.register_service(module.restart.clone());

        module
    }
}

/// Access to services, NPC, economy, and core services from a concrete assembled world.
pub trait ServicesContext: greyhack_world_npcs::NpcContext {
    /// Returns this world's services module.
    fn services(&self) -> Rc<ServicesModule>;
}
