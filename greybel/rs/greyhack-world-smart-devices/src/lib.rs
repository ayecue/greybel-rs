#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack smart-devices domain: network appliances and their accidents, police traffic
//! cameras and vehicle accidents, and browser CCTV feeds.
//!
//! [`SmartDevicesModule`] registers the domain schema, services, and persisted traffic state. The
//! composition root registers [`routes::register`] for its concrete world. Public entities, object
//! handles, failures, and services are re-exported at the crate root; repositories remain private
//! to the domain.

pub mod model;
pub mod objects;
pub mod routes;
pub mod services;
pub mod storage;

use std::rc::Rc;

pub use greyhack_behaviour::{Rng, SmartApplianceSettings};
use greyhack_world_core::WorldRef;
pub use greyhack_world_core::{ComputerId, NetworkId, PersonId, Result, WorldError};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use model::{CameraFeed, CameraNpc, LocatedVehicle, TrafficNet, TrafficState};
pub use objects::{ObjectTableExt, SmartApplianceHandle, TrafficNetHandle};
pub use services::{ApplianceFailure, Appliances, Cctv, CctvFailure, Traffic, TrafficFailure};
use storage::SmartDeviceRepository;

/// Services and smart-device storage of this domain.
#[derive(Clone)]
pub struct SmartDevicesModule {
    /// Appliance settings, overrides, alarms, and accidents.
    pub appliances: Rc<Appliances>,
    /// Police camera credentials, vehicle lookup, and traffic accidents.
    pub traffic: Rc<Traffic>,
    /// Browser camera feeds and external-access rules.
    pub cctv: Rc<Cctv>,
}

impl SmartDevicesModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let smart_devices = Rc::new(SmartDeviceRepository::new(world.db().clone()));
        let appliances = Rc::new(Appliances::new(
            world,
            smart_devices.clone(),
            registry.get(),
            registry.get(),
        ));
        let traffic = Rc::new(Traffic::new(registry, smart_devices.clone()));
        let cctv = Rc::new(Cctv::new(
            world,
            smart_devices,
            registry.get(),
            registry.get(),
            registry.get(),
        ));

        Self {
            appliances,
            traffic,
            cctv,
        }
    }

    /// Returns the smart-devices schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(<Self as Module>::NAME, [storage::schema::GROUP])
    }
}

impl Module for SmartDevicesModule {
    const NAME: &'static str = "smart-devices";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_capability::<dyn greyhack_world_core::PersistentState>(
            module.traffic.clone(),
        );
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.appliances.clone());
        registry.register_service(module.traffic.clone());
        registry.register_service(module.cctv.clone());

        module
    }
}

/// Access to smart-device, guild, NPC, economy, and core services from a concrete assembled world.
pub trait SmartDevicesContext: greyhack_world_guilds::GuildContext {
    /// Returns this world's smart-devices module.
    fn smart_devices(&self) -> Rc<SmartDevicesModule>;
}
