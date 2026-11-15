//! Library version changes that retain zones and weaknesses, implemented by libraries.

use crate::{CallFuture, ComputerId, WorldRef};

/// Library changes requested by core.
pub trait LibraryVersionService {
    /// Changes a library file's version while preserving its zones and weaknesses. Higher domain:
    /// libraries (`RedGlobal.ShouldRestore`).
    fn set_library_version<'a>(
        &'a self,
        world: &'a WorldRef,
        computer: ComputerId,
        path: &'a str,
        version: &'a str,
    ) -> CallFuture<'a, bool> {
        let _ = (world, computer, path, version);

        Box::pin(async { Ok(false) })
    }
}
