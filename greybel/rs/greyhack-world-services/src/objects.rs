//! Package-manager and computer-service object handles and typed object-table access.

use std::any::Any;

use greyhack_behaviour::ServiceId;
use greyhack_world_core::{Access, ComputerId, WorldError};
use greyhack_world_router::protocol::ObjectId;
use greyhack_world_router::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::routes::Result;

/// Service `service` reached through computer access `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceRef {
    /// Credentials and computer through which the service was opened.
    pub access: Access,
    /// Service opened by the script.
    pub service: ServiceId,
}

/// Package manager accessed with `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AptClientHandle(pub Access);

impl Handle<ComputerId> for AptClientHandle {
    const CLASS: ObjectClass = ObjectClass::AptClient;

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

/// Computer-service object handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceHandle(pub ServiceRef);

impl Handle<ComputerId> for ServiceHandle {
    const CLASS: ObjectClass = ObjectClass::Service;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Computer(self.0.access.computer))
    }
}

/// Typed services-object lookup and registration on a tree's object table.
pub trait ObjectTableExt {
    /// Returns package-manager access.
    fn apt_client(&self, id: ObjectId) -> Result<Access>;
    /// Returns service access.
    fn service(&self, id: ObjectId) -> Result<ServiceRef>;
    /// Registers a package manager.
    fn add_apt_client(&mut self, access: Access) -> ObjectId;
    /// Registers a computer service.
    fn add_service(&mut self, service: ServiceRef) -> ObjectId;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn apt_client(&self, id: ObjectId) -> Result<Access> {
        Ok(self.get::<AptClientHandle, WorldError>(id)?.0.clone())
    }

    fn service(&self, id: ObjectId) -> Result<ServiceRef> {
        Ok(self.get::<ServiceHandle, WorldError>(id)?.0.clone())
    }

    fn add_apt_client(&mut self, access: Access) -> ObjectId {
        self.insert(AptClientHandle(access))
    }

    fn add_service(&mut self, service: ServiceRef) -> ObjectId {
        self.insert(ServiceHandle(service))
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn apt_client(&self, id: ObjectId) -> Result<Access> {
        self.read(|objects| ObjectTableExt::apt_client(objects, id))
    }

    fn service(&self, id: ObjectId) -> Result<ServiceRef> {
        self.read(|objects| ObjectTableExt::service(objects, id))
    }

    fn add_apt_client(&mut self, access: Access) -> ObjectId {
        self.write(|objects| ObjectTableExt::add_apt_client(objects, access))
    }

    fn add_service(&mut self, service: ServiceRef) -> ObjectId {
        self.write(|objects| ObjectTableExt::add_service(objects, service))
    }
}
