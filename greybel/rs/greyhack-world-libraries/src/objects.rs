//! Library and Metaxploit script-object handles and typed object-table access.

use std::any::Any;

use greyhack_world_core::{Access, ComputerId, FileRef, RouterId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::routes::Result;

/// Library at `path` with access identity `access`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LibraryRef {
    pub access: Access,
    pub path: String,
    /// Computer whose metaxploit opened the library (`HelperValMetaLib.metaxploitComputerID`).
    pub origin: ComputerId,
}

/// `metaLib` target backed by a library file or router `kernel_router.so`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MetaLibRef {
    File(LibraryRef),
    Kernel(KernelRef),
}

/// Router `kernel_router.so` reached by `net_use` at `address` from `origin`.
/// See `HelperValMetaLib.routerAddress` and `metaxploitComputerID`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KernelRef {
    pub router: RouterId,
    pub address: String,
    pub origin: ComputerId,
    /// Bounces of the metaxploit that opened the session (`HelperValMetaLib.numBounces`).
    pub bounces: u32,
}

/// `metaxploit` access identity and library version used for scans and exploits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetaxploitRef {
    pub access: Access,
    pub version: String,
}

/// `net_use` target: a service library opened from `origin`, or a router.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NetSessionRef {
    Service { file: FileRef, origin: ComputerId },
    Router(KernelRef),
}

impl NetSessionRef {
    /// Returns the metaxploit computer that opened the session (`HelperValFile.prevBounceIP` stores its public address).
    #[must_use]
    pub const fn origin(&self) -> ComputerId {
        match self {
            Self::Service { origin, .. } | Self::Router(KernelRef { origin, .. }) => *origin,
        }
    }
}

macro_rules! handles {
    ($($name:ident($inner:ty) => $class:ident;)*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Eq, Hash)]
            pub struct $name(pub $inner);

            impl Handle<ComputerId> for $name {
                const CLASS: ObjectClass = ObjectClass::$class;

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
        )*
    };
}

handles! {
    MetaxploitHandle(MetaxploitRef) => Metaxploit;
    DebugLibraryHandle(LibraryRef) => DebugLibrary;
}

/// Library opened through metaxploit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MetaLibHandle(pub MetaLibRef);

impl Handle<ComputerId> for MetaLibHandle {
    const CLASS: ObjectClass = ObjectClass::MetaLib;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        match &self.0 {
            MetaLibRef::File(library) => Some(ObjectReach::Computer(library.access.computer)),
            MetaLibRef::Kernel(kernel) => Some(ObjectReach::Router(kernel.router.computer())),
        }
    }
}

/// Network session opened through metaxploit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetSessionHandle(pub NetSessionRef);

impl Handle<ComputerId> for NetSessionHandle {
    const CLASS: ObjectClass = ObjectClass::NetSession;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        match &self.0 {
            NetSessionRef::Service { file, .. } => {
                Some(ObjectReach::Computer(file.access.computer))
            }
            NetSessionRef::Router(kernel) => Some(ObjectReach::Router(kernel.router.computer())),
        }
    }
}

/// Typed library and Metaxploit object lookup on a tree's object table.
pub trait ObjectTableExt {
    /// Returns metaxploit access and version.
    fn metaxploit(&self, id: ObjectId) -> Result<MetaxploitRef>;
    /// Returns library access.
    fn meta_lib(&self, id: ObjectId) -> Result<MetaLibRef>;
    /// Returns network-session access.
    fn net_session(&self, id: ObjectId) -> Result<NetSessionRef>;
    /// Returns debug-library access.
    fn debug_library(&self, id: ObjectId) -> Result<LibraryRef>;
}

macro_rules! accessors {
    ($($name:ident => $handle:ident.0 as $result:ty;)*) => {
        $(
            fn $name(&self, id: ObjectId) -> Result<$result> {
                Ok(self.get::<$handle, WorldError>(id)?.0.clone())
            }
        )*
    };
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    accessors! {
        metaxploit => MetaxploitHandle.0 as MetaxploitRef;
        meta_lib => MetaLibHandle.0 as MetaLibRef;
        net_session => NetSessionHandle.0 as NetSessionRef;
        debug_library => DebugLibraryHandle.0 as LibraryRef;
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn metaxploit(&self, id: ObjectId) -> Result<MetaxploitRef> {
        self.read(|objects| ObjectTableExt::metaxploit(objects, id))
    }

    fn meta_lib(&self, id: ObjectId) -> Result<MetaLibRef> {
        self.read(|objects| ObjectTableExt::meta_lib(objects, id))
    }

    fn net_session(&self, id: ObjectId) -> Result<NetSessionRef> {
        self.read(|objects| ObjectTableExt::net_session(objects, id))
    }

    fn debug_library(&self, id: ObjectId) -> Result<LibraryRef> {
        self.read(|objects| ObjectTableExt::debug_library(objects, id))
    }
}
