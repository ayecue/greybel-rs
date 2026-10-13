use std::marker::PhantomData;
use std::rc::Rc;

use duplex_core::{Capabilities, Driver, OutboundBatch};
use scheduler::TokioScheduler;

use crate::client::Client;
use crate::downstream::Downstream;

/// HTTP stream window of 16,384 items, chosen to reduce credit requests.
pub const HTTP_STREAM_WINDOW: u32 = 16 * 1024;

/// Provider driver for one client HTTP transport with downstream mode `M`.
pub struct HttpDriver<M: Downstream> {
    client: Rc<Client>,
    scheduler: TokioScheduler,
    downstream: PhantomData<fn() -> M>,
}

impl<M: Downstream> HttpDriver<M> {
    pub(crate) fn new(client: Rc<Client>) -> Self {
        Self {
            client,
            scheduler: TokioScheduler::new(),
            downstream: PhantomData,
        }
    }
}

impl<M: Downstream> Driver for HttpDriver<M> {
    type Scheduler = TokioScheduler;

    const CAPABILITIES: Capabilities = Capabilities::DUPLEX.without(Capabilities::CLIENT);
    const STREAM_WINDOW: u32 = HTTP_STREAM_WINDOW;

    fn transport_name(&self) -> &'static str {
        M::TRANSPORT
    }

    fn send(&self, batch: OutboundBatch) {
        self.client.send(batch.as_slice());
    }

    fn scheduler(&self) -> &TokioScheduler {
        &self.scheduler
    }

    fn random(&self) -> [u8; 16] {
        duplex_tokio::random_bytes()
    }

    fn pause(&self) {
        self.client.read_gate.pause();
    }

    fn resume(&self) {
        self.client.read_gate.resume();
    }
}
