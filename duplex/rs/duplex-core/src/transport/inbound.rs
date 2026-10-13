use std::ops::Range;
use std::rc::Rc;

use super::TransportState;
use crate::connection_id::ConnectionId;
use crate::definition::{ErrorCode, Role};
use crate::driver::{Driver, InboundBatch};
use crate::frame::BatchReader;
use crate::keys::Work;

pub(crate) struct Inbound<'b> {
    pub(crate) batch: &'b Rc<[u8]>,
    pub(crate) frames: Range<usize>,
    pub(crate) connection: Option<ConnectionId>,
    pub(crate) core: u8,
}

pub(crate) trait Endpoint {
    fn receive(self: Rc<Self>, inbound: Inbound<'_>);
    fn process(&self, _work: Work) {}
    fn on_disconnect(&self);
    fn close(&self, reason: ErrorCode);
}

impl<D: Driver> TransportState<D> {
    pub(crate) fn receive(&self, batch: InboundBatch) {
        let Some(batch) = self.inbound(batch) else {
            return;
        };
        let offset = batch.range().start;
        let Ok(slices) = BatchReader::new(batch.as_slice()) else {
            return;
        };
        let batch = batch.buffer();
        let core = slices.core();

        self.hub.enter_cycle();
        for slice in slices {
            let Ok(slice) = slice else {
                break;
            };
            let Some(route) = self.remote_aliases.borrow_mut().route(slice.key) else {
                continue;
            };
            let local = match route.from {
                Role::Client => Role::Provider,
                Role::Provider => Role::Client,
            };
            let Some(endpoint) = self.find_endpoint(route.channel, local) else {
                continue;
            };

            endpoint.receive(Inbound {
                batch,
                frames: slice.frames.start + offset..slice.frames.end + offset,
                connection: route.connection,
                core,
            });
        }

        self.hub.process_read_batch();
        self.hub.exit_cycle();
    }

    pub(crate) fn on_disconnect(&self) {
        self.end(ErrorCode::ConnectionLoss);
    }
}
