use std::cell::RefCell;
use std::rc::Rc;

use slotmap::SlotMap;

use super::connection::{
    ConnectionActive, ConnectionEnd, ConnectionFailure, ConnectionInit, ConnectionState, Health,
};
use crate::keys::HookKey;
use crate::protocol::Protocol;

type Listener<S> = dyn Fn(&S);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Hook identifier accepted by `Channel::remove_hook`.
pub struct HookId {
    hook: Hook,
    handle: HookKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Hook {
    Open,
    Ack,
    Failure,
    Close,
    Health,
}

pub(crate) struct HookManager<Pr: Protocol> {
    open: HookTable<Listener<ConnectionInit>>,
    ack: HookTable<Listener<ConnectionActive<Pr>>>,
    failure: HookTable<Listener<ConnectionFailure<Pr>>>,
    close: HookTable<Listener<ConnectionEnd>>,
    health: HookTable<dyn Fn(Health)>,
}

impl<Pr: Protocol> Default for HookManager<Pr> {
    fn default() -> Self {
        Self {
            open: HookTable::default(),
            ack: HookTable::default(),
            failure: HookTable::default(),
            close: HookTable::default(),
            health: HookTable::default(),
        }
    }
}

impl<Pr: Protocol> HookManager<Pr> {
    pub(crate) fn on_open(&self, listener: Rc<Listener<ConnectionInit>>) -> HookId {
        Self::id(Hook::Open, self.open.add(listener))
    }

    pub(crate) fn on_ack(&self, listener: Rc<Listener<ConnectionActive<Pr>>>) -> HookId {
        Self::id(Hook::Ack, self.ack.add(listener))
    }

    pub(crate) fn on_failure(&self, listener: Rc<Listener<ConnectionFailure<Pr>>>) -> HookId {
        Self::id(Hook::Failure, self.failure.add(listener))
    }

    pub(crate) fn on_close(&self, listener: Rc<Listener<ConnectionEnd>>) -> HookId {
        Self::id(Hook::Close, self.close.add(listener))
    }

    pub(crate) fn on_health(&self, listener: Rc<dyn Fn(Health)>) -> HookId {
        Self::id(Hook::Health, self.health.add(listener))
    }

    pub(crate) fn remove(&self, id: HookId) {
        match id.hook {
            Hook::Open => self.open.remove(id.handle),
            Hook::Ack => self.ack.remove(id.handle),
            Hook::Failure => self.failure.remove(id.handle),
            Hook::Close => self.close.remove(id.handle),
            Hook::Health => self.health.remove(id.handle),
        }
    }

    pub(crate) fn enter(&self, state: &ConnectionState<Pr>) {
        match state {
            ConnectionState::Init(init) => self.open.snapshot().iter().for_each(|hook| hook(init)),
            ConnectionState::Active(active) => {
                self.ack.snapshot().iter().for_each(|hook| hook(active));
            }
            ConnectionState::Failure(failure) => {
                self.failure
                    .snapshot()
                    .iter()
                    .for_each(|hook| hook(failure));
            }
            ConnectionState::End(final_state) => {
                self.close
                    .snapshot()
                    .iter()
                    .for_each(|hook| hook(final_state));
            }
        }
    }

    pub(crate) fn health(&self, health: Health) {
        for hook in self.health.snapshot() {
            hook(health);
        }
    }

    fn id(hook: Hook, handle: HookKey) -> HookId {
        HookId { hook, handle }
    }
}

pub(crate) struct HookTable<L: ?Sized> {
    listeners: RefCell<SlotMap<HookKey, Rc<L>>>,
}

impl<L: ?Sized> Default for HookTable<L> {
    fn default() -> Self {
        Self {
            listeners: RefCell::default(),
        }
    }
}

impl<L: ?Sized> HookTable<L> {
    pub(crate) fn add(&self, listener: Rc<L>) -> HookKey {
        self.listeners.borrow_mut().insert(listener)
    }

    pub(crate) fn remove(&self, handle: HookKey) {
        self.listeners.borrow_mut().remove(handle);
    }

    pub(crate) fn snapshot(&self) -> Vec<Rc<L>> {
        self.listeners.borrow().values().cloned().collect()
    }
}
