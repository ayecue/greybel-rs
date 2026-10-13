use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::subscription::{Filter, Subscription};
use crate::protocol::GreybelNotification;

struct Subscriber {
    filter: Filter,
    tx: UnboundedSender<GreybelNotification>,
}

#[derive(Default)]
struct Registry {
    next_id: u64,
    subscribers: HashMap<u64, Subscriber>,
    closed: bool,
}

impl Registry {
    fn next_id(&mut self) -> u64 {
        let id = self.next_id;

        self.next_id += 1;

        id
    }
}

#[derive(Default)]
pub(crate) struct Hub {
    registry: Mutex<Registry>,
}

impl Hub {
    fn lock(&self) -> MutexGuard<'_, Registry> {
        self.registry.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn subscribe(self: &Arc<Self>, filter: Filter) -> Subscription {
        let (tx, rx) = unbounded_channel();
        let mut registry = self.lock();
        let id = registry.next_id();

        if !registry.closed {
            registry.subscribers.insert(id, Subscriber { filter, tx });
        }

        Subscription::new(rx, Arc::downgrade(self), id)
    }

    pub(super) fn unsubscribe(&self, id: u64) {
        self.lock().subscribers.remove(&id);
    }

    pub(crate) fn dispatch(&self, notification: &GreybelNotification) {
        self.lock().subscribers.retain(|_, subscriber| {
            !(subscriber.filter)(notification) || subscriber.tx.send(notification.clone()).is_ok()
        });
    }

    pub(crate) fn close(&self) {
        let mut registry = self.lock();

        registry.closed = true;
        registry.subscribers.clear();
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.lock().closed
    }
}
