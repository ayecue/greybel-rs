//! Network-administrator lookup used by core accounts.

use greyhack_world_effects::BoxFuture;

use crate::{NetworkId, PersonId, Result, WorldRef};

/// Network-administrator lookup for core accounts.
pub trait NetworkAdminLookup {
    /// Returns the person administering `network`, if any. Higher domain: npcs.
    fn network_admin_person<'a>(
        &'a self,
        world: &'a WorldRef,
        network: NetworkId,
    ) -> BoxFuture<'a, Result<Option<PersonId>>> {
        let _ = (world, network);

        Box::pin(async { Ok(None) })
    }
}
