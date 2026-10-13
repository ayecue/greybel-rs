use scheduler::Scheduler;

use super::TransportState;
use super::outbox::NextBatch;
use crate::binary::BinaryWriter;
use crate::channel::Bundle;
use crate::driver::Driver;
use crate::error::Error;
use crate::frame::{FrameEncoder, Route};

/// Initial outbound buffer capacity for one full batch and the reserved driver prefix.
const BATCH_CAPACITY: usize = 72 * 1024;

impl<D: Driver> TransportState<D> {
    pub(crate) fn write(
        &self,
        route: Route,
        bundle: Bundle,
        minor: u16,
        max_frame: usize,
        write: impl FnOnce(&mut FrameEncoder<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let full = {
            let mut outboxes = self.outboxes.borrow_mut();

            outboxes.write(route, bundle, minor, max_frame, write)?;

            outboxes.is_full()
        };

        if full {
            self.flush();
        } else {
            self.defer_flush();
        }

        Ok(())
    }

    pub(crate) fn release(&self, route: Route) {
        self.outboxes.borrow_mut().aliases.release(route);
    }

    pub(crate) fn flush(&self) {
        self.scheduled.set(false);
        if !self.can_send() {
            return;
        }

        let mut pass = std::mem::take(&mut *self.flush.borrow_mut());

        self.outboxes.borrow().begin_flush(&mut pass);
        loop {
            let mut batch = BinaryWriter::from_vec(self.batches.take(BATCH_CAPACITY));
            let next_batch = self.outboxes.borrow_mut().take_next(&mut pass, &mut batch);

            match next_batch {
                NextBatch::Ready { start } => {
                    if let Some(outbound_batch) = self.outbound(batch.into_bytes(), start) {
                        self.driver.send(outbound_batch);
                    }
                }
                NextBatch::Failed => self.batches.release(batch.into_bytes()),
                NextBatch::Done => {
                    self.batches.release(batch.into_bytes());
                    break;
                }
            }
        }

        self.outboxes.borrow_mut().end_flush();
        *self.flush.borrow_mut() = pass;
        if !self.outboxes.borrow().is_empty() {
            self.defer_flush();
        }
    }

    pub(crate) fn flush_all(&self) {
        while self.can_send() && !self.outboxes.borrow().is_empty() {
            self.flush();
        }
    }

    fn defer_flush(&self) {
        if self.scheduled.replace(true) {
            return;
        }

        let this = self.weak();

        self.driver.scheduler().defer(Box::new(move || {
            if let Some(state) = this.upgrade() {
                state.flush();
            }
        }));
    }
}
