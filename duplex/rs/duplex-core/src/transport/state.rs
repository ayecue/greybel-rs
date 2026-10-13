use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use scheduler::{Clock, Scheduler};

use super::Endpoint;
use super::aliases::RemoteAliasTable;
use super::endpoints::EndpointTable;
use super::outbox::{FlushState, OutboxContainer};
use super::security::TransportSecurity;
use super::work_queue::WorkQueue;
use crate::definition::{ErrorCode, Role};
use crate::driver::{BatchPool, Driver};
use crate::hub::{Hub, HubState, ProcessResult, Schedulable};
use crate::keys::{EndpointId, TransportId};

pub(crate) struct TransportState<D: Driver> {
    pub(super) this: Weak<Self>,
    pub(super) driver: D,
    endpoints: EndpointTable,
    pub(super) remote_aliases: RefCell<RemoteAliasTable>,
    pub(super) outboxes: RefCell<OutboxContainer>,
    pub(super) batches: Rc<BatchPool>,
    pub(super) flush: RefCell<FlushState>,
    pub(super) scheduled: Cell<bool>,
    pub(super) security: TransportSecurity,
    pub(super) work_queue: WorkQueue,
    pub(super) hub: Rc<HubState>,
    slot: Cell<Option<TransportId>>,
    pub(super) reason: Cell<Option<ErrorCode>>,
}

impl<D: Driver> TransportState<D> {
    pub(super) fn new(this: Weak<Self>, driver: D) -> Self {
        Self {
            this,
            driver,
            endpoints: EndpointTable::default(),
            remote_aliases: RefCell::default(),
            outboxes: RefCell::new(OutboxContainer::with_capacity(D::RESERVED_PREFIX_BYTES)),
            batches: Rc::default(),
            flush: RefCell::default(),
            scheduled: Cell::new(false),
            security: TransportSecurity::default(),
            work_queue: WorkQueue::default(),
            hub: Rc::clone(Hub::current().state()),
            slot: Cell::new(None),
            reason: Cell::new(None),
        }
    }

    pub(super) fn register(self: &Rc<Self>) {
        let this: Weak<dyn Schedulable> = Rc::downgrade(self) as Weak<dyn Schedulable>;

        self.slot.set(Some(self.hub.register(this)));
    }

    pub(super) fn slot(&self) -> TransportId {
        self.slot
            .get()
            .expect("a transport registers with its hub when made")
    }

    pub(crate) fn driver(&self) -> &D {
        &self.driver
    }

    pub(crate) fn weak(&self) -> Weak<Self> {
        Weak::clone(&self.this)
    }

    pub(crate) fn reason(&self) -> Option<ErrorCode> {
        self.reason.get()
    }

    pub(crate) fn end(&self, reason: ErrorCode) {
        if self.reason.get().is_some() {
            return;
        }

        self.reason.set(Some(reason));
        self.remote_aliases.borrow_mut().clear();
        self.outboxes.borrow_mut().clear();
        self.clear_work_queue();
        for endpoint in self.endpoints() {
            match reason {
                ErrorCode::ConnectionLoss => endpoint.on_disconnect(),
                reason => endpoint.close(reason),
            }
        }
    }

    pub(crate) fn attach<E: Endpoint + 'static>(
        &self,
        channel: u32,
        role: Role,
        endpoint: impl FnOnce(EndpointId) -> Rc<E>,
    ) -> Rc<E> {
        self.endpoints.attach(channel, role, endpoint)
    }

    pub(crate) fn detach(&self, handle: EndpointId) {
        self.endpoints.detach(handle);
    }

    pub(crate) fn find_endpoint(&self, channel: u32, role: Role) -> Option<Rc<dyn Endpoint>> {
        self.endpoints.find(channel, role)
    }

    pub(crate) fn endpoint(&self, handle: EndpointId) -> Option<Rc<dyn Endpoint>> {
        self.endpoints.get(handle)
    }

    pub(crate) fn endpoints(&self) -> Vec<Rc<dyn Endpoint>> {
        self.endpoints.all()
    }
}

impl<D: Driver> Schedulable for TransportState<D> {
    fn process_next(&self, most: usize) -> ProcessResult {
        Self::process_next(self, most)
    }

    fn defer(&self, task: Box<dyn FnOnce()>) {
        self.driver.scheduler().defer(task);
    }

    fn now(&self) -> Duration {
        self.driver.scheduler().now()
    }
}

impl<D: Driver> Drop for TransportState<D> {
    fn drop(&mut self) {
        if let Some(slot) = self.slot.get() {
            self.hub.unregister(slot);
        }
    }
}
