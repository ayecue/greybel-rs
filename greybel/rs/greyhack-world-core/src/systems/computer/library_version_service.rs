//! Library version changes that retain zones and weaknesses, implemented by libraries.

use greyhack_world_effects::BoxFuture;

use crate::{ComputerId, Result, WorldRef};

/// Library changes requested by core hardware operations.
pub trait LibraryVersionService {
    /// Changes a library file's version while preserving its zones and weaknesses. Higher domain:
    /// libraries (`RedGlobal.ShouldRestore`).
    fn set_library_version<'a>(
        &'a self,
        world: &'a WorldRef,
        computer: ComputerId,
        path: &'a str,
        version: &'a str,
    ) -> BoxFuture<'a, Result<bool>> {
        let _ = (world, computer, path, version);

        Box::pin(async { Ok(false) })
    }
}
