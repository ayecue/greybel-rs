//! NPC identity and current-state lookup used by core accounts and networks.

use crate::{CallFuture, ComputerId, PersonId, WorldRef};

/// Which NPC [`NpcState::npc_state`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcLookup<'a> {
    /// The NPC that is `person`.
    Person(PersonId),
    /// The NPC that owns account `user` on `computer`.
    Account {
        /// Computer with the account.
        computer: ComputerId,
        /// Account name.
        user: &'a str,
    },
}

/// How an NPC stands at the world's current second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcStatus {
    /// Whether the NPC is dead (`NpcInfo.IsDead`).
    pub dead: bool,
    /// Whether the NPC is dead or fired from its job.
    pub dead_or_fired: bool,
    /// Whether the NPC is logged in (`Persona.IsLogged`).
    pub online: bool,
    /// Whether the NPC is at work by its schedule.
    pub at_work: bool,
}

/// Looks up NPC state for core networks.
pub trait NpcState {
    /// Returns how the NPC `lookup` stands, or `None` when there is no such NPC. Higher domain:
    /// npcs.
    fn npc_state<'a>(
        &'a self,
        world: &'a WorldRef,
        lookup: NpcLookup<'a>,
    ) -> CallFuture<'a, Option<NpcStatus>>;
}
