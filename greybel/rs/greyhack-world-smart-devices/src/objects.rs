//! Smart-device script-object handles and typed object-table access.

use std::any::Any;

use greyhack_world_core::{Access, ComputerId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::routes::Result;

/// Smart appliance accessed with `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SmartApplianceHandle(pub Access);

impl Handle<ComputerId> for SmartApplianceHandle {
    const CLASS: ObjectClass = ObjectClass::SmartAppliance;

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

/// Police traffic-network library accessed with `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrafficNetHandle(pub Access);

impl Handle<ComputerId> for TrafficNetHandle {
    const CLASS: ObjectClass = ObjectClass::TrafficNet;

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

/// Typed smart-device object lookup on a tree's object table.
pub trait ObjectTableExt {
    /// Returns smart-appliance access.
    fn smart_appliance(&self, id: ObjectId) -> Result<Access>;
    /// Returns traffic-network access.
    fn traffic_net(&self, id: ObjectId) -> Result<Access>;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn smart_appliance(&self, id: ObjectId) -> Result<Access> {
        Ok(self.get::<SmartApplianceHandle, WorldError>(id)?.0.clone())
    }

    fn traffic_net(&self, id: ObjectId) -> Result<Access> {
        Ok(self.get::<TrafficNetHandle, WorldError>(id)?.0.clone())
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn smart_appliance(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| ObjectTableExt::smart_appliance(objects, id))
    }

    fn traffic_net(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| ObjectTableExt::traffic_net(objects, id))
    }
}
