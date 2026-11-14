use std::any::Any;

use greyhack_world_programs::{Handle, ObjectClass, ObjectIdentity, ObjectReach};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct HandleId(pub u32);

impl Handle<u32> for HandleId {
    const CLASS: ObjectClass = ObjectClass::File;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<u32>> {
        Some(ObjectReach::Computer(self.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Other(pub u32);

impl Handle<u32> for Other {
    const CLASS: ObjectClass = ObjectClass::Computer;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<u32>> {
        Some(ObjectReach::Computer(self.0))
    }
}
