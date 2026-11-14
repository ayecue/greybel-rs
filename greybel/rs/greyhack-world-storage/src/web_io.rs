use std::sync::Arc;

use scheduler::{Clock as _, WebScheduler};
use turso::core::{
    Clock, File, IO, MemoryIO, MonotonicInstant, OpenFlags, Result, WallClockInstant,
};

/// Turso in-memory storage using `WebScheduler` because Turso's clock panics on
/// `wasm32-unknown-unknown`. Wall-clock time remains zero and host calendar time never affects the
/// world. Persistence occurs only at controlled commit points. The single connection does not wait
/// on busy locks or query timeouts.
#[derive(Default)]
pub(super) struct WebIo(MemoryIO);

impl WebIo {
    pub(super) fn default_io() -> Arc<dyn IO> {
        Arc::new(Self::default())
    }
}

impl Clock for WebIo {
    fn current_time_monotonic(&self) -> MonotonicInstant {
        MonotonicInstant::from_nanos(WebScheduler.now().as_nanos())
    }

    fn current_time_wall_clock(&self) -> WallClockInstant {
        WallClockInstant { secs: 0, micros: 0 }
    }
}

impl IO for WebIo {
    fn open_file(&self, path: &str, flags: OpenFlags, direct: bool) -> Result<Arc<dyn File>> {
        self.0.open_file(path, flags, direct)
    }

    fn remove_file(&self, path: &str) -> Result<()> {
        self.0.remove_file(path)
    }

    fn supports_shared_wal_coordination(&self) -> bool {
        self.0.supports_shared_wal_coordination()
    }
}
