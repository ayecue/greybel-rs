//! SMTP access-object resolution supplied by the intrusion domain.

use greyhack_world_core::{Access, ComputerId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{ObjectTableHandle, RouteResult};

/// Resolves the `crypto.so` object that requests an SMTP user list.
pub trait SmtpAccess {
    /// Returns the computer access for `object` in `objects`.
    fn access(
        &self,
        objects: &ObjectTableHandle<ComputerId>,
        object: ObjectId,
    ) -> RouteResult<Access, WorldError>;
}
