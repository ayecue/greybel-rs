#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack creation domain: deterministic network and LAN plans, computer and device drafts,
//! random files and mail, network NPCs, procedural-mission placement, and administrator restores.
//!
//! [`CreationModule`] registers the domain's empty schema and services, including core's lazy
//! network-generation service and the NPC domain's network-creation and restore services. The composition
//! root registers [`routes::register`] for its concrete world. Generation drafts, layout entities,
//! object-table helpers, and services are re-exported at the crate root; creation owns no tables.

pub mod generation;
pub mod model;
mod network_creation;
pub mod objects;
pub mod routes;
pub mod services;

use std::rc::Rc;

pub use generation::{
    ComputerOptions, LanPlan, NetworkOptions, RepositoryDraft, WifiGenerator, add_player_libraries,
    assign_player_libraries, build_filesystem, kernel_router, router_computer, system_processes,
};
use greyhack_world_core::WorldRef;
use greyhack_world_modules::{Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use model::{DeviceLayout, ForwardLayout, LanNodeLayout, NetworkLayout};
pub use objects::ObjectTableExt;
use services::zero_day::ZeroDayMissions;
pub use services::{MissionPlacement, NetworkGeneration, RestoreService};

/// Creation services built from core, NPC, services, libraries, and smart-device services.
#[derive(Clone)]
pub struct CreationModule {
    /// Lazy and explicit creation of networks, computers, and devices.
    pub network_generation: Rc<NetworkGeneration>,
    /// Procedural-mission accounts, files, logs, and mail.
    pub mission_placement: Rc<MissionPlacement>,
    /// Administrator restoration of generated networks.
    pub restores: Rc<RestoreService>,
    /// Deterministic zero-day and procedural mission fixtures.
    pub zero_day: Rc<ZeroDayMissions>,
}

impl CreationModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();

        Self {
            mission_placement: Rc::new(MissionPlacement::new(world, registry)),
            network_generation: Rc::new(NetworkGeneration::new(registry)),
            restores: Rc::new(RestoreService::new(registry)),
            zero_day: Rc::new(ZeroDayMissions::new(
                world,
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
            )),
        }
    }

    /// Returns the creation schema, which owns no tables.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(<Self as Module>::NAME, [])
    }
}

impl Module for CreationModule {
    const NAME: &'static str = "creation";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_service::<dyn greyhack_world_core::NetworkGeneration>(
            module.network_generation.clone(),
        );
        registry.register_service::<dyn greyhack_world_npcs::NetworkCreation>(
            module.network_generation.clone(),
        );
        registry
            .register_service::<dyn greyhack_world_npcs::NetworkRestore>(module.restores.clone());
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.network_generation.clone());
        registry.register_service(module.mission_placement.clone());
        registry.register_service(module.restores.clone());

        module
    }
}
