//! One ordered queue of typed domain events.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// Future returned by an asynchronous event subscriber.
pub type EventFuture<'a> = Pin<Box<dyn Future<Output = Result<(), EventError>> + 'a>>;

/// Error returned by an asynchronous domain event subscriber.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EventError(Box<dyn std::error::Error>);

impl EventError {
    /// Erases a concrete subscriber error.
    pub fn new(error: impl std::error::Error + 'static) -> Self {
        Self(Box::new(error))
    }
}

/// A domain event dispatched at the end of a world cycle.
pub trait Event: Any + 'static {}

type Subscriber = Rc<dyn Fn(&dyn Any)>;
type AsyncSubscriber = Rc<dyn Fn(&dyn Any) -> EventFuture<'_>>;
type SubscriberMap = RefCell<HashMap<TypeId, Vec<Subscriber>>>;
type AsyncSubscriberMap = RefCell<HashMap<TypeId, Vec<AsyncSubscriber>>>;

/// Published events and their assembly-time subscribers.
pub struct EventQueue {
    queue: RefCell<Vec<Box<dyn Any>>>,
    subscribers: SubscriberMap,
    async_subscribers: AsyncSubscriberMap,
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "EventQueue debug output omits type-erased events and subscriber callbacks"
)]
impl std::fmt::Debug for EventQueue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventQueue")
            .field("queued", &self.queue.borrow().len())
            .finish()
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self {
            queue: RefCell::new(Vec::new()),
            subscribers: RefCell::new(HashMap::new()),
            async_subscribers: RefCell::new(HashMap::new()),
        }
    }
}

impl EventQueue {
    /// Publishes `event` after every event already queued.
    pub fn publish<E: Event>(&self, event: E) {
        self.queue.borrow_mut().push(Box::new(event));
    }

    /// Publishes `event` directly to its subscribers without changing the queued event order.
    ///
    /// This is for boundaries whose subscriber effects must be visible before world work continues.
    pub async fn publish_now<E: Event>(&self, event: E) -> Result<(), EventError> {
        let event: Box<dyn Any> = Box::new(event);
        let subscribers = self
            .subscribers
            .borrow()
            .get(&event.as_ref().type_id())
            .cloned()
            .unwrap_or_default();

        for subscriber in subscribers {
            subscriber(event.as_ref());
        }

        let subscribers = self
            .async_subscribers
            .borrow()
            .get(&event.as_ref().type_id())
            .cloned()
            .unwrap_or_default();

        for subscriber in subscribers {
            subscriber(event.as_ref()).await?;
        }

        Ok(())
    }

    /// Subscribes `subscriber` to events of type `E`.
    pub fn on<E: Event>(&self, subscriber: impl Fn(&E) + 'static) {
        self.subscribers
            .borrow_mut()
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Rc::new(move |event| {
                subscriber(
                    event
                        .downcast_ref::<E>()
                        .expect("event subscribers are indexed by type"),
                );
            }));
    }

    /// Dispatches the currently queued events to their subscribers in publish order. Events published by
    /// subscribers wait for the next dispatch, as they do with a drained cycle queue.
    pub fn dispatch(&self) {
        let queue = std::mem::take(&mut *self.queue.borrow_mut());

        for event in queue {
            let subscribers = self
                .subscribers
                .borrow()
                .get(&event.as_ref().type_id())
                .cloned()
                .unwrap_or_default();

            for subscriber in subscribers {
                subscriber(event.as_ref());
            }
        }
    }

    /// Subscribes an asynchronous domain handler to `E`.
    pub fn on_async<E: Event>(
        &self,
        subscriber: impl for<'a> Fn(&'a E) -> EventFuture<'a> + 'static,
    ) {
        self.async_subscribers
            .borrow_mut()
            .entry(TypeId::of::<E>())
            .or_default()
            .push(Rc::new(move |event| {
                subscriber(
                    event
                        .downcast_ref::<E>()
                        .expect("event subscribers are indexed by type"),
                )
            }));
    }

    /// Dispatches queued events in publish order, awaiting asynchronous subscribers in place.
    pub async fn dispatch_async(&self) -> Result<(), EventError> {
        let queue = std::mem::take(&mut *self.queue.borrow_mut());

        for event in queue {
            let subscribers = self
                .subscribers
                .borrow()
                .get(&event.as_ref().type_id())
                .cloned()
                .unwrap_or_default();

            for subscriber in subscribers {
                subscriber(event.as_ref());
            }

            let subscribers = self
                .async_subscribers
                .borrow()
                .get(&event.as_ref().type_id())
                .cloned()
                .unwrap_or_default();

            for subscriber in subscribers {
                subscriber(event.as_ref()).await?;
            }
        }

        Ok(())
    }

    /// Returns the number of queued events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.borrow().len()
    }

    /// Returns whether no event is queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}
