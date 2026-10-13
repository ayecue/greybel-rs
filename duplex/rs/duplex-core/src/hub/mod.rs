mod cycle;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

use slotmap::SlotMap;

use crate::keys::TransportId;

pub(crate) struct ProcessResult {
    pub(crate) count: usize,
    pub(crate) has_next: bool,
}

pub(crate) trait Schedulable {
    fn process_next(&self, most: usize) -> ProcessResult;
    fn defer(&self, task: Box<dyn FnOnce()>);
    fn now(&self) -> Duration;
}

/// Schedules message processing for every transport on a thread.
#[derive(Clone)]
pub struct Hub(Rc<HubState>);

pub(crate) struct HubState {
    this: Weak<Self>,
    transports: RefCell<SlotMap<TransportId, Weak<dyn Schedulable>>>,
    queue: RefCell<VecDeque<TransportId>>,
    incoming_count: Cell<usize>,
    depth: Cell<u32>,
    running: Cell<bool>,
    scheduled: Cell<bool>,
}

thread_local! {
    static CURRENT: Rc<HubState> = Rc::new_cyclic(|this| HubState {
        this: Weak::clone(this),
        transports: RefCell::default(),
        queue: RefCell::default(),
        incoming_count: Cell::new(0),
        depth: Cell::new(0),
        running: Cell::new(false),
        scheduled: Cell::new(false),
    });
}

impl Hub {
    #[must_use]
    /// Returns the current thread hub.
    pub fn current() -> Self {
        Self(CURRENT.with(Rc::clone))
    }

    #[must_use]
    /// Returns registered transport count.
    pub fn transports(&self) -> usize {
        self.0.transports.borrow().len()
    }

    /// Returns the received-message processing backlog across registered transports.
    #[must_use]
    pub fn incoming_count(&self) -> usize {
        self.0.incoming_count.get()
    }

    pub(crate) fn state(&self) -> &Rc<HubState> {
        &self.0
    }
}

impl HubState {
    pub(crate) fn register(&self, transport: Weak<dyn Schedulable>) -> TransportId {
        self.transports.borrow_mut().insert(transport)
    }

    pub(crate) fn unregister(&self, transport: TransportId) {
        self.transports.borrow_mut().remove(transport);
    }

    fn transport(&self, handle: TransportId) -> Option<Rc<dyn Schedulable>> {
        self.transports.borrow().get(handle).and_then(Weak::upgrade)
    }

    fn weak(&self) -> Weak<Self> {
        Weak::clone(&self.this)
    }
}
