#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack creation domain: deterministic network and LAN plans, computer and device drafts,
//! random files and mail, network NPCs, procedural-mission placement, and administrator restores.
//!
//! [`CreationModule`] registers the domain's empty schema and services, including core's lazy
//! network-generation hook and the NPC domain's network-creation and restore hooks. The composition
//! root registers [`routes::register`] for its concrete world. Generation drafts, layout entities,
//! object-table helpers, and services are re-exported at the crate root; creation owns no tables.

pub mod generation;
mod hooks;
pub mod model;
pub mod objects;
pub mod routes;
pub mod services;

use std::rc::Rc;

pub use generation::{
    ComputerOptions, LanPlan, NetworkOptions, RepositoryDraft, WifiGenerator, add_player_libraries,
    assign_player_libraries, build_filesystem, kernel_router, router_computer, system_processes,
};
pub use greyhack_behaviour::{JobRole, NetworkAdmin, Rng, SmartApplianceSettings, Word};
use greyhack_world_core::WorldRef;
pub use greyhack_world_core::{
    Computer, ComputerId, Contact, Gender, Hardware, LanPlace, LayoutFailure, NetCard,
    NetworkDraft, NetworkId, Node, NodeId, Part, PersonId, Port, Result, Router, RouterId, User,
    WorldError,
};
pub use greyhack_world_libraries::{
    ComputerDraft, FileSystemDraft, Library, NodeDraft, RouterDraft,
};
use greyhack_world_modules::{Module, Registry};
pub use greyhack_world_npcs::Npc;
pub use greyhack_world_services::{
    Crime, Grade, Offense, Package, Record, RecordDatabase, RecordKind, Repository, Subject,
};
use greyhack_world_storage::DomainSchema;
pub use model::{DeviceLayout, ForwardLayout, LanNodeLayout, NetworkLayout};
pub use objects::ObjectTableExt;
pub use services::{MissionPlacement, NetworkGeneration, Restores};

/// Creation services built from core, NPC, services, libraries, and smart-device services.
#[derive(Clone)]
pub struct CreationModule {
    /// Lazy and explicit creation of networks, computers, and devices.
    pub network_generation: Rc<NetworkGeneration>,
    /// Procedural-mission accounts, files, logs, and mail.
    pub mission_placement: Rc<MissionPlacement>,
    /// Administrator restoration of generated networks.
    pub restores: Rc<Restores>,
}

impl CreationModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();

        Self {
            mission_placement: Rc::new(MissionPlacement::new(world, registry)),
            network_generation: Rc::new(NetworkGeneration::new(registry)),
            restores: Rc::new(Restores::new(registry)),
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

/// Access to creation and all lower domain services used by its routes.
pub trait CreationContext:
    greyhack_world_libraries::LibraryContext + greyhack_world_services::ServicesContext
{
    /// Returns this world's creation module.
    fn creation(&self) -> Rc<CreationModule>;
}
