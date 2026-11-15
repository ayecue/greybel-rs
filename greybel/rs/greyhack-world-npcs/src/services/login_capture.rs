//! Credential capture requested by NPC social-engineering logins.

use greyhack_world_core::{ComputerId, Result};
use greyhack_world_effects::BoxFuture;

/// Records the credential packet exposed by an NPC social-engineering login.
pub trait NpcLoginCapture {
    /// Records the login on sniffers along its path before forcing the NPC online
    /// (`Persona.ProcesaIngenieraSocial`).
    fn record_npc_login<'a>(
        &'a self,
        computer: ComputerId,
        user: &'a str,
    ) -> BoxFuture<'a, Result<()>>;
}
