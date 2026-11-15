use greyhack_world_core::{Router, RouterId};

/// Returns the ID of the router found by a search.
pub fn id(router: Option<Router>) -> Option<RouterId> {
    router.map(|router| router.id)
}
