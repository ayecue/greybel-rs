//! Persons: records and the account owner of a computer.

use greyhack_world_core::{ComputerId, Dep, NetworkId, Person, PersonId, WorldRef};
use greyhack_world_economy::Bank;

use crate::{Mail, NetworkAdmin, NetworkCreation, Npc, Result};

/// NPC people, accounts, phones, and network administrators.
pub struct NpcService {
    pub(crate) world: WorldRef,
    pub(crate) npcs: std::rc::Rc<crate::storage::NpcRepository>,
    pub(crate) network_admins: std::rc::Rc<crate::storage::NetworkAdminRepository>,
    pub(crate) computers: Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: Dep<greyhack_world_core::FileSystems>,
    pub(crate) mail: Dep<Mail>,
    pub(crate) bank: Dep<Bank>,
    pub(crate) network_creation: Dep<dyn NetworkCreation>,
    pub(crate) scheduler: std::rc::Rc<greyhack_world_routines::Scheduler>,
}

pub(crate) struct NpcServiceSupport {
    pub(crate) npcs: std::rc::Rc<crate::storage::NpcRepository>,
    pub(crate) network_admins: std::rc::Rc<crate::storage::NetworkAdminRepository>,
    pub(crate) computers: Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: Dep<greyhack_world_core::FileSystems>,
    pub(crate) mail: Dep<Mail>,
    pub(crate) bank: Dep<Bank>,
    pub(crate) scheduler: std::rc::Rc<greyhack_world_routines::Scheduler>,
    pub(crate) network_creation: Dep<dyn NetworkCreation>,
}

impl NpcService {
    pub(crate) fn new(world: &WorldRef, dependencies: NpcServiceSupport) -> Self {
        Self {
            world: world.clone(),
            npcs: dependencies.npcs,
            network_admins: dependencies.network_admins,
            computers: dependencies.computers,
            files: dependencies.files,
            mail: dependencies.mail,
            bank: dependencies.bank,
            network_creation: dependencies.network_creation,
            scheduler: dependencies.scheduler,
        }
    }

    /// Returns a base person identity.
    pub async fn person(&self, id: PersonId) -> Result<Option<Person>> {
        self.computers.person(id).await
    }

    /// Saves a base person identity.
    pub async fn save_person(&self, person: &Person) -> Result<()> {
        self.computers.save_person(person).await
    }

    /// Returns the person who owns an account on `computer`.
    pub async fn person_on(&self, computer: ComputerId) -> Result<Option<PersonId>> {
        self.computers.person_on(computer).await
    }

    /// Writes the NPC job, schedule, and work-performance fields.
    pub async fn save_npc_job(&self, npc: &Npc) -> Result<()> {
        self.npcs.save_npc_job(npc).await
    }

    /// Returns the NPCs whose computers belong to `network`.
    pub async fn npcs_in(&self, network: NetworkId) -> Result<Vec<Npc>> {
        self.npcs.npcs_in(network).await
    }

    /// Returns the administrator record of `network`, if any.
    pub async fn network_admin(&self, network: NetworkId) -> Result<Option<NetworkAdmin>> {
        self.network_admins.network_admin(network).await
    }

    /// Returns the person administering `network`, if any.
    pub async fn network_admin_person(&self, network: NetworkId) -> Result<Option<PersonId>> {
        self.network_admins.network_admin_person(network).await
    }

    /// Sets the administrator record and optional person assigned to `network`.
    pub async fn set_network_admin(
        &self,
        network: NetworkId,
        person: Option<PersonId>,
        admin: &NetworkAdmin,
    ) -> Result<()> {
        self.network_admins
            .set_network_admin(network, person, admin)
            .await
    }
}

impl greyhack_world_core::NpcHook for NpcService {
    fn npc_state<'a>(
        &'a self,
        world: &'a WorldRef,
        lookup: greyhack_world_core::NpcLookup<'a>,
    ) -> greyhack_world_core::HookFuture<'a, Option<greyhack_world_core::NpcState>> {
        Box::pin(async move {
            let npc = match lookup {
                greyhack_world_core::NpcLookup::Person(person) => self.npc(person).await?,
                greyhack_world_core::NpcLookup::Account { computer, user } => {
                    self.npc_on_computer(computer, user).await?
                }
            };
            let now = world.world().time_seconds;

            Ok(npc.map(|npc| greyhack_world_core::NpcState {
                dead: npc.is_dead(now),
                dead_or_fired: npc.is_dead_or_fired(now),
                online: npc.online,
                at_work: npc.schedule.is_at_work(now),
            }))
        })
    }
}

impl greyhack_world_core::NetworkAdminHook for NpcService {
    fn network_admin_person<'a>(
        &'a self,
        _: &'a WorldRef,
        network: NetworkId,
    ) -> greyhack_world_core::HookFuture<'a, Option<PersonId>> {
        Box::pin(async move { self.network_admin_person(network).await })
    }
}
