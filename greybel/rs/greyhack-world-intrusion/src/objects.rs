//! Cryptography script-object handles and typed object-table access.

use std::any::Any;

use greyhack_world_core::{Access, ComputerId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::routes::Result;

/// Cryptography library accessed with `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CryptoHandle(pub Access);

impl Handle<ComputerId> for CryptoHandle {
    const CLASS: ObjectClass = ObjectClass::Crypto;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Computer(self.0.computer))
    }
}

/// Typed cryptography-object lookup on a tree's object table.
pub trait ObjectTableExt {
    /// Returns cryptography access.
    fn crypto(&self, id: ObjectId) -> Result<Access>;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn crypto(&self, id: ObjectId) -> Result<Access> {
        Ok(self.get::<CryptoHandle, WorldError>(id)?.0.clone())
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn crypto(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| ObjectTableExt::crypto(objects, id))
    }
}
