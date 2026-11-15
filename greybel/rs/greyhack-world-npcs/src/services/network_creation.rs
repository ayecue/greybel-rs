//! Network creation and administrator restoration used by NPC services.

use greyhack_world_core::{ComputerId, PersonId, Result, RouterId};
use greyhack_world_effects::BoxFuture;

/// Network and phone creation used by NPC accounts, people, and rental administrators.
pub trait NetworkCreation {
    /// Creates a network at `public_ip` for generated NPC content (`ServerMap.SpawnRouter`).
    fn add_network<'a>(&'a self, public_ip: &'a str) -> BoxFuture<'a, Result<RouterId>>;

    /// Creates an NPC’s phone with its initial account (`NetworkLan.AddSmartPhone`).
    fn add_phone<'a>(
        &'a self,
        owner: ComputerId,
        npc: PersonId,
        user: &'a str,
        password: &'a str,
    ) -> BoxFuture<'a, Result<Option<ComputerId>>>;
}

/// Restores an administrator's network from the NPC scheduler.
pub trait NetworkRestore {
    /// `Router.AdminRestoreNetwork`: restores the administrator's network.
    fn restore_admin_network(&self, admin_computer: ComputerId) -> BoxFuture<'_, Result<()>>;
}
