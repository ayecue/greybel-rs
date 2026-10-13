use std::cell::RefCell;

use duplex_core::{Capabilities, Driver, OutboundBatch, Transport, WeakTransport};
use scheduler::Scheduler;

/// Driver for one in-memory transport endpoint. A deferred task on the supplied scheduler delivers outbound batches
/// to the peer.
pub struct MemoryDriver<S: Scheduler> {
    scheduler: S,
    peer: RefCell<Option<WeakTransport<Self>>>,
    /// Generates non-cryptographic IDs for distinguishing transport connections.
    connection_id_rng: RefCell<fastrand::Rng>,
}

impl<S: Scheduler> MemoryDriver<S> {
    /// Creates connected transports using `scheduler` for deferred delivery, timers, and time.
    #[must_use]
    pub fn pair(scheduler: S) -> (Transport<Self>, Transport<Self>) {
        let seed = u64::try_from(scheduler.now().as_nanos()).unwrap_or(u64::MAX);
        let first = Transport::new(|_| Self {
            scheduler: scheduler.clone(),
            peer: RefCell::new(None),
            connection_id_rng: RefCell::new(fastrand::Rng::with_seed(seed)),
        });
        let second = Transport::new(|_| Self {
            scheduler,
            peer: RefCell::new(Some(first.downgrade())),
            connection_id_rng: RefCell::new(fastrand::Rng::with_seed(!seed)),
        });

        first.driver().peer.replace(Some(second.downgrade()));

        (first, second)
    }
}

impl<S: Scheduler> Driver for MemoryDriver<S> {
    type Scheduler = S;

    const CAPABILITIES: Capabilities = Capabilities::DUPLEX;

    fn transport_name(&self) -> &'static str {
        "memory"
    }

    fn send(&self, batch: OutboundBatch) {
        let Some(peer) = self.peer.borrow().clone() else {
            return;
        };
        let bytes = batch.into_vec();

        self.scheduler.defer(Box::new(move || peer.receive(bytes)));
    }

    fn scheduler(&self) -> &S {
        &self.scheduler
    }

    fn random(&self) -> [u8; 16] {
        let mut connection_id_rng = self.connection_id_rng.borrow_mut();

        std::array::from_fn(|_| connection_id_rng.u8(..))
    }
}

impl<S: Scheduler> Drop for MemoryDriver<S> {
    /// Schedules peer transport disconnection notification.
    fn drop(&mut self) {
        if let Some(peer) = self.peer.get_mut().take() {
            self.scheduler.defer(Box::new(move || peer.on_disconnect()));
        }
    }
}
