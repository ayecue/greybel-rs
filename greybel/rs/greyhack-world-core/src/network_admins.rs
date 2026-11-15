//! Network-administrator lookup used by core accounts.

use crate::{CallFuture, NetworkId, PersonId, WorldRef};

/// Network-administrator lookup for core accounts.
pub trait NetworkAdminLookup {
    /// Returns the person administering `network`, if any. Higher domain: npcs.
    fn network_admin_person<'a>(
        &'a self,
        world: &'a WorldRef,
        network: NetworkId,
    ) -> CallFuture<'a, Option<PersonId>> {
        let _ = (world, network);

        Box::pin(async { Ok(None) })
    }
}
