//! Calls from NPC services and routes to higher domains.

use greyhack_world_core::{Access, ComputerId, HookFuture, PersonId, RouterId, WorldError};
use greyhack_world_router::protocol::ObjectId;
use greyhack_world_router::{ObjectTableHandle, RouteResult};

/// Network and phone creation used by NPC accounts, people, and rental administrators.
pub trait NetworkCreation {
    /// Creates a network at `public_ip` for generated NPC content (`ServerMap.SpawnRouter`).
    fn add_network<'a>(&'a self, public_ip: &'a str) -> HookFuture<'a, RouterId>;

    /// Creates an NPC’s phone with its initial account (`NetworkLan.AddSmartPhone`).
    fn add_phone<'a>(
        &'a self,
        owner: ComputerId,
        npc: PersonId,
        user: &'a str,
        password: &'a str,
    ) -> HookFuture<'a, Option<ComputerId>>;
}

/// Restores an administrator's network from the NPC scheduler.
pub trait NetworkRestore {
    /// `Router.AdminRestoreNetwork`: restores the administrator's network.
    fn restore_admin_network(&self, admin_computer: ComputerId) -> HookFuture<'_, ()>;
}

/// Records the credential packet exposed by an NPC social-engineering login.
pub trait NpcLoginCapture {
    /// Records the login on sniffers along its path before forcing the NPC online
    /// (`Persona.ProcesaIngenieraSocial`).
    fn record_npc_login<'a>(&'a self, computer: ComputerId, user: &'a str) -> HookFuture<'a, ()>;
}

/// Resolves the `crypto.so` object that requests an SMTP user list.
pub trait SmtpAccess {
    /// Returns the computer access for `object` in `objects`.
    fn access(
        &self,
        objects: &ObjectTableHandle<ComputerId>,
        object: ObjectId,
    ) -> RouteResult<Access, WorldError>;
}
