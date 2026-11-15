#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack NPC domain: people and network administrators, employment, work schedules and
//! phone alarms, mail accounts and messages, and generated NPC inboxes.
//!
//! [`NpcModule`] registers the domain schema, services, routine kind, and lazy dependencies. The
//! composition root registers [`routes::register`] for its concrete world and implements the
//! domain's upward traits for creation, guilds, and intrusion. Public entities and services are
//! re-exported at the crate root; repositories remain private to the domain.

pub mod generation;
pub mod model;
pub mod objects;
pub mod routes;
pub mod routines;
pub mod services;
pub mod storage;

use std::rc::Rc;

pub use greyhack_behaviour::{Rng, Word};
pub use greyhack_world_core::{ComputerId, NetworkId, PersonId, Result, RouterId, WorldError};
use greyhack_world_core::{Dep, WorldRef};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_routines::Routine;
use greyhack_world_storage::DomainSchema;
pub use model::*;
pub use objects::{MailRef, MetaMailHandle, ObjectTableExt};
pub use routines::{NpcRoutineState, NpcSwitch};
pub use services::*;
use storage::{MailRepository, NetworkAdminRepository, NpcRepository};

/// Services and repositories of the NPC domain.
#[derive(Clone)]
pub struct NpcModule {
    /// NPC people, accounts, phones, and network administrators.
    pub npcs: Rc<NpcService>,
    /// NPC employment and rented-server administrators.
    pub jobs: Rc<JobService>,
    /// NPC work schedules, logins, lateness, and alarms.
    pub schedules: Rc<ScheduleService>,
    /// Mail accounts, messages, NPC inboxes, and SMTP users.
    pub mail: Rc<Mail>,
    /// NPC login and logout routine.
    pub switch: Rc<NpcSwitch>,
    pub(crate) smtp_access: Dep<dyn SmtpAccess>,
}

impl NpcModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let scheduler = registry.scheduler();
        let db = world.db().clone();
        let npcs = Rc::new(NpcRepository::new(db.clone()));
        let network_admins = Rc::new(NetworkAdminRepository::new(db.clone()));
        let mail = Rc::new(MailRepository::new(db, registry.get()));

        let mail_service = Rc::new(Mail::new(
            world,
            mail,
            npcs.clone(),
            network_admins.clone(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));
        let npc_service = Rc::new(NpcService::new(
            world,
            NpcServiceSupport {
                npcs,
                network_admins,
                computers: registry.get(),
                files: registry.get(),
                mail: registry.get(),
                bank: registry.get(),
                scheduler: scheduler.clone(),
                network_creation: registry.get(),
            },
        ));
        let jobs = Rc::new(JobService::new(world, registry, scheduler.clone()));
        let schedules = Rc::new(ScheduleService::new(world, registry, scheduler.clone()));

        Self {
            npcs: npc_service,
            jobs,
            schedules,
            mail: mail_service,
            switch: Rc::new(NpcSwitch::new(world, registry.get(), scheduler)),
            smtp_access: registry.get(),
        }
    }

    /// Returns the NPC schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [
                storage::schema::mail::GROUP,
                storage::schema::npcs::GROUP,
                NpcSwitch::tables(),
                storage::schema::VOLATILE,
            ],
        )
    }
}

impl Module for NpcModule {
    const NAME: &'static str = "npcs";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_service::<dyn greyhack_world_core::NpcState>(module.npcs.clone());
        registry
            .register_service::<dyn greyhack_world_core::NetworkAdminLookup>(module.npcs.clone());
        registry.register_routine(module.switch.clone());
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.npcs.clone());
        registry.register_service(module.jobs.clone());
        registry.register_service(module.schedules.clone());
        registry.register_service(module.mail.clone());
        registry.register_service(module.switch.clone());

        module
    }
}
