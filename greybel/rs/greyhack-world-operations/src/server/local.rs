//! In-process game-server transports.

use duplex_core::Transport;
use duplex_driver_memory::MemoryDriver;
use scheduler::Scheduler;

use super::WorldServer;

impl WorldServer {
    /// Returns a transport with the server attached to its peer. A same-thread runtime opens its world
    /// channel on this transport, and `scheduler` runs tasks and timers for both ends.
    #[must_use]
    pub fn connect<S: Scheduler>(&self, scheduler: S) -> Transport<MemoryDriver<S>> {
        let (program, server) = MemoryDriver::pair(scheduler);

        self.attach(&server);

        program
    }

    /// Calls [`WorldServer::connect`] with the current Tokio `LocalSet`, without a host transport.
    #[cfg(not(target_family = "wasm"))]
    #[must_use]
    pub fn connect_on_tokio(&self) -> Transport<MemoryDriver<scheduler::TokioScheduler>> {
        self.connect(scheduler::TokioScheduler::new())
    }
}
