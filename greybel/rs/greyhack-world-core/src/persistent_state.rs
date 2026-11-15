//! Domain state saved and loaded with core metadata, implemented by smart devices.

use crate::{CallFuture, WorldRef};

/// Domain state persisted with core metadata.
pub trait PersistentState {
    /// Loads domain-owned persisted state after core loads `world_meta`. Higher domain: smart
    /// devices for the traffic cooldown.
    fn load_state<'a>(&'a self, world: &'a WorldRef) -> CallFuture<'a, ()> {
        let _ = world;

        Box::pin(async { Ok(()) })
    }

    /// Writes domain-owned persisted state when core commits `world_meta`. Higher domain: smart
    /// devices for the traffic cooldown.
    fn save_state<'a>(&'a self, world: &'a WorldRef) -> CallFuture<'a, ()> {
        let _ = world;

        Box::pin(async { Ok(()) })
    }
}
