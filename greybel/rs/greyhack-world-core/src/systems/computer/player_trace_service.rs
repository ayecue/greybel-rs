//! The player's trace token in log entries, implemented by traces.

use greyhack_world_effects::BoxFuture;

use crate::{Result, WorldRef};

/// Player traces triggered by core actions.
pub trait PlayerTraceService {
    /// Returns the token the player's log entries carry now. Higher domain: traces.
    fn player_token_trace<'a>(&'a self, world: &'a WorldRef) -> BoxFuture<'a, Result<String>>;
}
