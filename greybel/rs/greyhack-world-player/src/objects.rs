//! CTF script-object handles and typed object-table access.

use std::any::Any;

use greyhack_world_core::{ComputerId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::routes::Result;

/// CTF event `event` obtained on `computer`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CtfRef {
    /// Computer through which the event was opened.
    pub computer: ComputerId,
    /// Name of the event.
    pub event: String,
}

/// Capture-the-flag event. Event identity excludes the computer used to open it (`HelperScript.AddCTF`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CtfHandle(pub CtfRef);

impl Handle<ComputerId> for CtfHandle {
    const CLASS: ObjectClass = ObjectClass::Ctf;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, &self.0.event)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self.0.event == other.0.event)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Computer(self.0.computer))
    }
}

/// Typed CTF-object lookup and registration on a tree's object table.
pub trait ObjectTableExt {
    /// Returns CTF access.
    fn ctf(&self, id: ObjectId) -> Result<CtfRef>;
    /// Registers a CTF event opened on its computer.
    fn add_ctf(&mut self, ctf: CtfRef) -> ObjectId;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn ctf(&self, id: ObjectId) -> Result<CtfRef> {
        Ok(self.get::<CtfHandle, WorldError>(id)?.0.clone())
    }

    fn add_ctf(&mut self, ctf: CtfRef) -> ObjectId {
        self.insert(CtfHandle(ctf))
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn ctf(&self, id: ObjectId) -> Result<CtfRef> {
        self.read(|objects| ObjectTableExt::ctf(objects, id))
    }

    fn add_ctf(&mut self, ctf: CtfRef) -> ObjectId {
        self.write(|objects| ObjectTableExt::add_ctf(objects, ctf))
    }
}
