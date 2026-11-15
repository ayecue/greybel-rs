#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack libraries domain: library zones and weaknesses, version releases and reports,
//! Metaxploit scans and sessions, overflow rewards, debug libraries, and the exploit shop.
//!
//! [`LibraryModule`] registers the domain schema, services, version-check routine, and the
//! capabilities used by core and intrusion. The composition root registers [`routes::register`]
//! for its concrete world. Public entities, object handles, failures, and services are re-exported
//! at the crate root; repositories remain private to the domain.

pub mod generation;
pub mod model;
pub mod objects;
pub mod routes;
pub mod routines;
pub mod services;
pub mod storage;

use std::rc::Rc;

pub use generation::{LibraryGeneration, VersionTable, WeaknessContext};
use greyhack_world_core::WorldRef;
use greyhack_world_effects::Routine;
use greyhack_world_modules::{Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use model::*;
pub use objects::{
    DebugLibraryHandle, KernelRef, LibraryRef, MetaLibHandle, MetaLibRef, MetaxploitHandle,
    MetaxploitRef, NetSessionHandle, NetSessionRef, ObjectTableExt,
};
pub use routines::LibraryCheck;
pub use services::debug::{DebugFailure, DebugLibraryPatch, DebugPayload, PartialObject};
pub use services::library_updates::LibraryReportFailure;
pub use services::library_versions::WeaknessReportFailure;
pub use services::metaxploit::rshell::{
    RshellClient, RshellClientFailure, RshellRemoveFailure, RshellServerFailure,
};
pub use services::metaxploit::scan::{LibraryScan, ZoneScan};
pub use services::report::{
    AttackResult, AttackReward, ExploitLog, ExploitOutcome, ExploitReport, UnmetRequirement,
};
pub use services::shop::failure::ExploitShopFailure;
pub use services::shop::purchase::ExploitPurchase;
pub use services::{
    Debug, ExploitService, Generator, LibraryService, Metaxploit, OverflowService, Shop,
    UpdateService, VersionService,
};
use storage::{
    DebugRepository, ExploitRepository, LibraryRepository, LibraryVersionRepository,
    ReverseShellRepository,
};

/// Services and repositories of the libraries domain.
#[derive(Clone)]
pub struct LibraryModule {
    /// Generated-draft persistence with full library entities.
    pub libraries: Rc<LibraryService>,
    /// Library files, zones, and weaknesses.
    pub generator: Rc<Generator>,
    /// Version releases, weakness uses, and exploit reports.
    pub versions: Rc<VersionService>,
    /// Metaxploit scans, sessions, and reverse shells.
    pub metaxploit: Rc<Metaxploit>,
    /// Overflow requirements, rewards, and logs.
    pub overflows: Rc<OverflowService>,
    /// Exploit scripts and descriptions.
    pub exploits: Rc<ExploitService>,
    /// Debug accounts, libraries, and zero-days.
    pub debug: Rc<Debug>,
    /// Exploit catalog, coupons, and purchases.
    pub shop: Rc<Shop>,
    /// Network-reported library updates.
    pub updates: Rc<UpdateService>,
    /// Scheduled library-version check.
    pub check: Rc<LibraryCheck>,
}

impl LibraryModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let scheduler = registry.scheduler();
        let db = world.db().clone();
        let libraries = Rc::new(LibraryRepository::new(
            db.clone(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));
        let library_versions = Rc::new(LibraryVersionRepository::new(db.clone()));
        let exploits = Rc::new(ExploitRepository::new(db.clone()));
        let debug = Rc::new(DebugRepository::new(db.clone()));
        let reverse_shells = Rc::new(ReverseShellRepository::new(db));

        let library_service = Rc::new(LibraryService::new(libraries.clone()));
        let generator = Rc::new(Generator::new(libraries.clone(), registry.get()));
        let shop = Rc::new(Shop::new(registry, exploits.clone(), libraries.clone()));
        let versions = Rc::new(VersionService::new(
            world,
            libraries.clone(),
            library_versions,
            registry.get(),
            registry.get(),
            registry.get(),
        ));
        let updates = Rc::new(UpdateService::new(
            world,
            libraries.clone(),
            registry.get(),
            registry.get(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));

        Self {
            libraries: library_service,
            generator: generator.clone(),
            versions: versions.clone(),
            metaxploit: Rc::new(Metaxploit::new(
                world,
                reverse_shells,
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
            )),
            overflows: Rc::new(OverflowService::new(registry)),
            exploits: Rc::new(ExploitService::new(exploits)),
            debug: Rc::new(Debug::new(world, debug, registry.get(), registry.get())),
            shop,
            updates,
            check: Rc::new(LibraryCheck::new(world, registry.get(), scheduler)),
        }
    }

    /// Returns the libraries schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [
                storage::schema::GROUP,
                storage::schema::COUPONS,
                LibraryCheck::tables(),
                storage::schema::VOLATILE,
            ],
        )
        .with_dependencies(&["core"])
    }
}

impl Module for LibraryModule {
    const NAME: &'static str = "libraries";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_service::<dyn greyhack_world_core::LibraryVersionService>(
            module.generator.clone(),
        );
        registry.register_service::<dyn greyhack_world_intrusion::LibraryObjectAccess>(
            module.metaxploit.clone(),
        );
        registry.register_routine(module.check.clone());
        registry.register_schema(Self::schema());
        registry.register_service(module.check.clone());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.libraries.clone());
        registry.register_service(module.generator.clone());
        registry.register_service(module.versions.clone());
        registry.register_service(module.metaxploit.clone());
        registry.register_service(module.overflows.clone());
        registry.register_service(module.exploits.clone());
        registry.register_service(module.debug.clone());
        registry.register_service(module.shop.clone());
        registry.register_service(module.updates.clone());

        module
    }
}
