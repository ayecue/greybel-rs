mod batch;
mod batch_stream;
mod inbound_buffers;

pub(crate) use batch::BatchPool;
pub use batch::{InboundBatch, OutboundBatch};
pub use batch_stream::BatchStream;
pub use inbound_buffers::InboundBufferManager;
use scheduler::Scheduler;

use crate::channel::DEFAULT_STREAM_WINDOW;
use crate::definition::Role;

/// Transport backend supplying channel I/O, scheduling, and randomness.
pub trait Driver: 'static {
    /// Scheduler for tasks, timers, and current time.
    type Scheduler: Scheduler;
    /// Driver-supported roles and transport modes.
    const CAPABILITIES: Capabilities;
    /// Default channel stream window.
    const STREAM_WINDOW: u32 = DEFAULT_STREAM_WINDOW;
    /// Bytes reserved before each outbound batch for driver framing.
    const RESERVED_PREFIX_BYTES: usize = 0;

    /// Returns the transport name used to select protocol flavors.
    fn transport_name(&self) -> &'static str;

    /// Sends one batch in call order.
    fn send(&self, batch: OutboundBatch);
    /// Returns the task, timer, and clock scheduler.
    fn scheduler(&self) -> &Self::Scheduler;
    /// Returns 16 random bytes for IDs; not suitable for cryptographic keys.
    fn random(&self) -> [u8; 16];
    /// Pauses reads until [`resume`](Self::resume). Drivers without pause support may keep reading.
    fn pause(&self) {}
    /// Resumes reads after [`pause`](Self::pause).
    fn resume(&self) {}
}

/// Scheduler timer type for `D`.
pub type TimerOf<D> = <<D as Driver>::Scheduler as Scheduler>::Timer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Driver role and transport-mode capability bitmask.
pub struct Capabilities(u8);

impl Capabilities {
    /// Can host clients.
    pub const CLIENT: Self = Self(0b001);
    /// Supports client and provider roles with persistent transports.
    pub const DUPLEX: Self = Self::CLIENT.with(Self::PROVIDER).with(Self::PERSISTENT);
    /// No capabilities.
    pub const NONE: Self = Self(0);
    /// Supports persistent transports for later batches.
    pub const PERSISTENT: Self = Self(0b100);
    /// Can host providers.
    pub const PROVIDER: Self = Self(0b010);

    #[must_use]
    /// Returns the union of both sets.
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    #[must_use]
    /// Returns capabilities present in `self` but absent from `other`.
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }

    #[must_use]
    /// Returns `true` when `self` contains all capabilities in `other`.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    /// Returns `true` when this set hosts `role` and contains all capabilities in `requires`.
    pub const fn supports(self, role: Role, requires: Self) -> bool {
        let hosts = match role {
            Role::Client => Self::CLIENT,
            Role::Provider => Self::PROVIDER,
        };

        self.contains(hosts.with(requires))
    }
}
