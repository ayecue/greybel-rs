//! Network generation on lookup, implemented by creation.

use crate::{CallFuture, RouterId, WorldRef};

/// Generates networks reached by core.
pub trait NetworkGeneration {
    /// Returns the main router of the network at `public_ip`, generating the network when the
    /// address belongs to one that was not made yet. Higher domain: creation.
    fn router_or_generate<'a>(
        &'a self,
        world: &'a WorldRef,
        public_ip: &'a str,
    ) -> CallFuture<'a, Option<RouterId>>;
}
