//! Library-owned object identities that intrusion routes need during an operation.

use std::future::Future;
use std::pin::Pin;

use greyhack_world_core::{Access, ComputerId, WorldError};
use greyhack_world_router::protocol::ObjectId;
use greyhack_world_router::{ObjectTableHandle, RouteResult};

/// Future returned by an asynchronous library-object lookup.
pub type ObjectAccessFuture<'a, T> = Pin<Box<dyn Future<Output = RouteResult<T, WorldError>> + 'a>>;

/// Resolves metaxploit and network-session objects owned by the libraries domain.
pub trait LibraryObjectAccess {
    /// Returns the access identity of a metaxploit for `MetaExploitIntrinsics.sniffer`.
    fn metaxploit_access(
        &self,
        objects: &ObjectTableHandle<ComputerId>,
        object: ObjectId,
    ) -> RouteResult<Access, WorldError>;

    /// Returns the target and origin computers of a network session for
    /// `NetSessionIntrinsics.flood_connection`, or `None` when its target no longer exists.
    fn flood_session<'a>(
        &'a self,
        objects: &'a ObjectTableHandle<ComputerId>,
        object: ObjectId,
    ) -> ObjectAccessFuture<'a, Option<(ComputerId, ComputerId)>>;
}
