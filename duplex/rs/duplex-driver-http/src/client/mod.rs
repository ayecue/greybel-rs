mod backlog;
mod request;
mod transport_id;

use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::pin::pin;
use std::task::Poll;
use std::time::Duration;

pub(crate) use backlog::Backlog;
use bytes::Bytes;
use duplex_core::InboundBatch;
use duplex_tokio::ReadGate;
pub(crate) use request::{ClientRequest, TransportClaim};
pub use request::{RECEIVED_HEADER, TRANSPORT_HEADER};
use tokio::sync::Notify;
use tokio::time::{self, Instant, MissedTickBehavior};
pub(crate) use transport_id::TransportId;

type Receive = Box<dyn Fn(InboundBatch)>;

/// Server state for one client transport: batch backlog, pending requests, and liveness.
pub(crate) struct Client {
    backlog: RefCell<Backlog>,
    batch_signal: Notify,
    /// Highest sequence returned by POST. Long polls skip it unless the client reports a gap.
    last_response_batch: Cell<u64>,
    post_count: Cell<usize>,
    streams: Cell<usize>,
    /// Set on request arrival and cleared by liveness sweeps.
    active: Cell<bool>,
    idle_for: Cell<Duration>,
    receive: RefCell<Option<Receive>>,
    disconnect: RefCell<Option<Box<dyn Fn()>>>,
    pub(crate) read_gate: ReadGate,
}

impl Client {
    pub(crate) fn new(max_backlog: usize) -> Self {
        Self {
            backlog: RefCell::new(Backlog::new(max_backlog)),
            batch_signal: Notify::new(),
            last_response_batch: Cell::new(0),
            post_count: Cell::new(0),
            streams: Cell::new(0),
            active: Cell::new(true),
            idle_for: Cell::new(Duration::ZERO),
            receive: RefCell::default(),
            disconnect: RefCell::default(),
            read_gate: ReadGate::default(),
        }
    }

    pub(crate) fn connect(&self, receive: Receive, disconnect: Box<dyn Fn()>) {
        self.receive.replace(Some(receive));
        self.disconnect.replace(Some(disconnect));
    }

    pub(crate) fn receive(&self, batch: InboundBatch) {
        if let Some(receive) = &*self.receive.borrow() {
            receive(batch);
        }
    }

    pub(crate) fn disconnect(&self) {
        let disconnect = self.disconnect.borrow_mut().take();

        self.receive.replace(None);
        if let Some(disconnect) = disconnect {
            disconnect();
        }
    }

    pub(crate) fn send(&self, batch: &[u8]) {
        if self.backlog.borrow_mut().push(batch) {
            self.batch_signal.notify_waiters();
        }
    }

    pub(crate) fn backlog(&self) -> std::cell::Ref<'_, Backlog> {
        self.backlog.borrow()
    }

    pub(crate) fn acknowledge(&self, sequence: u64) {
        self.backlog.borrow_mut().acknowledge(sequence);
    }

    /// Lowers the stored POST response sequence when the client reports an older `last_batch`, so lost batches can be
    /// retransmitted.
    pub(crate) fn note_only(&self, last_batch: u64) {
        if last_batch < self.last_response_batch.get() {
            self.last_response_batch.set(last_batch);
        }
    }

    /// Returns POST response records after `last_batch` and advances the sequence skipped by later long polls.
    pub(crate) fn deliver_after(&self, last_batch: u64) -> Option<Bytes> {
        let backlog = self.backlog.borrow();
        let (records, last) = backlog.records_after(last_batch)?;

        self.last_response_batch
            .set(self.last_response_batch.get().max(last));

        Some(Bytes::copy_from_slice(records))
    }

    /// Waits up to `timeout` for a post-`last_batch` sequence available to a long poll. Withholds new batches during
    /// POST handling. Returns the long poll's starting sequence.
    pub(crate) async fn poll_after(&self, last_batch: u64, timeout: Duration) -> u64 {
        let deadline = Instant::now() + timeout;

        loop {
            let batch_wait = self.batch_signal.notified();
            let after = last_batch.max(self.last_response_batch.get());

            if self.post_count.get() == 0 && self.backlog.borrow().has_after(after) {
                return after;
            }

            if time::timeout_at(deadline, batch_wait).await.is_err() {
                return last_batch.max(self.last_response_batch.get());
            }
        }
    }

    /// Waits for a backlog batch after `last_batch` or the next heartbeat tick.
    pub(crate) async fn stream_after(&self, last_batch: u64, heartbeat: &mut time::Interval) {
        let mut batch_wait = pin!(self.batch_signal.notified());

        if self.backlog.borrow().has_after(last_batch) {
            return;
        }

        poll_fn(|context| {
            if batch_wait.as_mut().poll(context).is_ready()
                || heartbeat.poll_tick(context).is_ready()
            {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }

    pub(crate) fn heartbeat(period: Duration) -> time::Interval {
        let mut ticks = time::interval_at(Instant::now() + period, period);

        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);

        ticks
    }

    pub(crate) fn begin_post(&self) -> PostGuard<'_> {
        self.post_count.set(self.post_count.get() + 1);

        PostGuard(self)
    }

    pub(crate) fn begin_stream(&self) -> StreamGuard<'_> {
        self.streams.set(self.streams.get() + 1);

        StreamGuard(self)
    }

    pub(crate) fn on_request(&self) {
        self.active.set(true);
    }

    /// Advances liveness by `period` and returns whether the client is overloaded or exceeded `idle` without requests
    /// or an open stream.
    pub(crate) fn should_disconnect_after(&self, period: Duration, idle: Duration) -> bool {
        if self.active.replace(false) || self.streams.get() > 0 {
            self.idle_for.set(Duration::ZERO);
        } else {
            self.idle_for.set(self.idle_for.get() + period);
        }

        self.backlog.borrow().is_overloaded() || self.idle_for.get() > idle
    }
}

pub(crate) struct PostGuard<'c>(&'c Client);

impl Drop for PostGuard<'_> {
    fn drop(&mut self) {
        let client = self.0;

        client.post_count.set(client.post_count.get() - 1);
        client.batch_signal.notify_waiters();
    }
}

pub(crate) struct StreamGuard<'c>(&'c Client);

impl Drop for StreamGuard<'_> {
    fn drop(&mut self) {
        let client = self.0;

        client.streams.set(client.streams.get() - 1);
        client.on_request();
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
