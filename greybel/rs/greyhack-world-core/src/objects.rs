//! Base script object handles and their game identity rules.

use std::any::Any;

use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects,
    ObjectTableHandle, RouteError, RouteResult,
};

use crate::{ComputerId, NodeId, RouterId, WorldError};

/// Object table used by computer-based program trees.
pub type ObjectTable = TreeObjects<ComputerId>;

/// Computer and user credentials for an object (`HelperValComputer`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Access {
    pub computer: ComputerId,
    pub user: String,
    /// Marks FTP access, which exposes fewer methods.
    pub ftp: bool,
    /// Public address of the computer the player came from (`HelperValComputer.prevBounceIP`).
    pub prev_ip: String,
    /// Number of connections between the player's computer and this one
    /// (`HelperValComputer.numBounces`).
    pub bounces: u32,
}

/// File `node` accessed as `access`, with its last observed `path` (`HelperValFile.pathFile`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileRef {
    pub access: Access,
    pub node: NodeId,
    pub path: String,
}

/// Whether computer access also has shell methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    /// Shell access.
    Shell,
    /// Computer-only access.
    Computer,
}

/// Shell or computer access, which share one game identity (`HelperScript.AddComputer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessHandle {
    /// Computer and user credentials.
    pub access: Access,
    /// Methods exposed by this registration.
    pub kind: AccessKind,
}

impl Handle<ComputerId> for AccessHandle {
    const CLASS: ObjectClass = ObjectClass::Computer;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, &self.access)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self.access == other.access)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Computer(self.access.computer))
    }

    fn replaces(&self) -> bool {
        self.kind == AccessKind::Shell
    }
}

/// Reverse-shell client shell, with one object per process (`Shell.reversePID`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReverseShellHandle(pub Access, pub u32);

impl Handle<ComputerId> for ReverseShellHandle {
    const CLASS: ObjectClass = ObjectClass::Shell;

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

/// File object handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileHandle(pub FileRef);

impl Handle<ComputerId> for FileHandle {
    const CLASS: ObjectClass = ObjectClass::File;

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

/// Router object handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RouterHandle(pub RouterId);

impl Handle<ComputerId> for RouterHandle {
    const CLASS: ObjectClass = ObjectClass::Router;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, self)
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>() == Some(self)
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Router(self.0.computer()))
    }
}

/// Typed base-object lookup on a connection's object table.
pub trait ObjectTableExt {
    /// Returns file access.
    fn file(&self, id: ObjectId) -> RouteResult<FileRef, WorldError>;
    /// Returns mutable file access for path updates.
    fn file_mut(&mut self, id: ObjectId) -> RouteResult<FileRef, WorldError>;
    /// Replaces file access after a synchronous path update.
    fn set_file(&mut self, id: ObjectId, file: FileRef) -> RouteResult<(), WorldError>;
    /// Returns router identity.
    fn router(&self, id: ObjectId) -> RouteResult<RouterId, WorldError>;
    /// Returns shell access, including reverse shells.
    fn shell(&self, id: ObjectId) -> RouteResult<Access, WorldError>;
    /// Returns a reverse shell's access and client process.
    fn reverse_shell(&self, id: ObjectId) -> RouteResult<ReverseShellHandle, WorldError>;
    /// Returns computer access from either a computer object or a shell.
    fn computer(&self, id: ObjectId) -> RouteResult<Access, WorldError>;
    /// Returns the computer and user access for a script session.
    fn session(&self, id: ObjectId) -> RouteResult<Access, WorldError>;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn file(&self, id: ObjectId) -> RouteResult<FileRef, WorldError> {
        Ok(self.get::<FileHandle, WorldError>(id)?.0.clone())
    }

    fn file_mut(&mut self, id: ObjectId) -> RouteResult<FileRef, WorldError> {
        Ok(self.get_mut::<FileHandle, WorldError>(id)?.0.clone())
    }

    fn set_file(&mut self, id: ObjectId, file: FileRef) -> RouteResult<(), WorldError> {
        self.get_mut::<FileHandle, WorldError>(id)?.0 = file;

        Ok(())
    }

    fn router(&self, id: ObjectId) -> RouteResult<RouterId, WorldError> {
        Ok(self.get::<RouterHandle, WorldError>(id)?.0)
    }

    fn shell(&self, id: ObjectId) -> RouteResult<Access, WorldError> {
        match self.get::<AccessHandle, WorldError>(id) {
            Ok(handle) if handle.kind == AccessKind::Shell => return Ok(handle.access.clone()),
            Ok(_) => return Err(RouteError::UnknownObject),
            Err(RouteError::UnknownObject) => {}
            Err(error) => return Err(error),
        }

        Ok(self.get::<ReverseShellHandle, WorldError>(id)?.0.clone())
    }

    fn reverse_shell(&self, id: ObjectId) -> RouteResult<ReverseShellHandle, WorldError> {
        Ok(self.get::<ReverseShellHandle, WorldError>(id)?.clone())
    }

    fn computer(&self, id: ObjectId) -> RouteResult<Access, WorldError> {
        match self.get::<AccessHandle, WorldError>(id) {
            Ok(handle) => return Ok(handle.access.clone()),
            Err(RouteError::UnknownObject) => {}
            Err(error) => return Err(error),
        }

        Ok(self.get::<ReverseShellHandle, WorldError>(id)?.0.clone())
    }

    fn session(&self, id: ObjectId) -> RouteResult<Access, WorldError> {
        self.shell(id)
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn file(&self, id: ObjectId) -> RouteResult<FileRef, WorldError> {
        self.read(|objects| objects.file(id))
    }

    fn file_mut(&mut self, id: ObjectId) -> RouteResult<FileRef, WorldError> {
        self.write(|objects| objects.file_mut(id))
    }

    fn set_file(&mut self, id: ObjectId, file: FileRef) -> RouteResult<(), WorldError> {
        self.write(|objects| objects.set_file(id, file))
    }

    fn router(&self, id: ObjectId) -> RouteResult<RouterId, WorldError> {
        self.read(|objects| objects.router(id))
    }

    fn shell(&self, id: ObjectId) -> RouteResult<Access, WorldError> {
        self.read(|objects| objects.shell(id))
    }

    fn reverse_shell(&self, id: ObjectId) -> RouteResult<ReverseShellHandle, WorldError> {
        self.read(|objects| objects.reverse_shell(id))
    }

    fn computer(&self, id: ObjectId) -> RouteResult<Access, WorldError> {
        self.read(|objects| objects.computer(id))
    }

    fn session(&self, id: ObjectId) -> RouteResult<Access, WorldError> {
        self.read(|objects| objects.session(id))
    }
}
