//! Registration of router and computer objects returned by creation routes.

use greyhack_world_core::{Access, AccessHandle, AccessKind, ComputerId, RouterHandle, RouterId};
use greyhack_world_router::ObjectTableHandle;
use greyhack_world_router::protocol::ObjectId;

/// Registers objects produced by network creation.
pub trait ObjectTableExt {
    /// Registers `router` as a router object.
    fn add_router(&mut self, router: RouterId) -> ObjectId;
    /// Registers root `access` to a created computer.
    fn add_computer(&mut self, access: Access) -> ObjectId;
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn add_router(&mut self, router: RouterId) -> ObjectId {
        self.insert(RouterHandle(router))
    }

    fn add_computer(&mut self, access: Access) -> ObjectId {
        self.insert(AccessHandle {
            access,
            kind: AccessKind::Computer,
        })
    }
}
