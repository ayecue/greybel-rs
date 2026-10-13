use std::any::Any;
use std::rc::Rc;

use super::NotificationDispatch as Dispatch;
use super::pipeline::{HandlerContainer, Pipeline};
use crate::protocol::NotificationMessage;

type Handler<N> = dyn for<'a> Fn(<N as NotificationMessage>::Ref<'a>, NextNotification<N>);

/// Remaining notification handlers in the chain.
pub struct NextNotification<N: NotificationMessage> {
    handlers: HandlerContainer<Handler<N>>,
    index: usize,
}

impl<N: NotificationMessage> NextNotification<N> {
    fn first(handlers: HandlerContainer<Handler<N>>) -> Self {
        Self { handlers, index: 0 }
    }

    /// Invokes the next handler.
    pub fn call(self, notification: N::Ref<'_>) {
        let Some(handler) = self.handlers.get(self.index).cloned() else {
            return;
        };
        let next = Self {
            handlers: self.handlers,
            index: self.index + 1,
        };

        handler(notification, next);
    }
}

pub(crate) struct NotificationHandler {
    dispatch: Dispatch,
    chain: Rc<dyn Any>,
}

impl NotificationHandler {
    pub(crate) fn new<N: NotificationMessage + 'static>() -> Self {
        let chain: Rc<Pipeline<Handler<N>>> = Rc::default();
        let route = Rc::clone(&chain);

        Self {
            dispatch: Rc::new(move |body| {
                if let Ok(notification) = body.decode_ref::<N::Ref<'_>>() {
                    NextNotification::first(route.handlers()).call(notification);
                }
            }),
            chain,
        }
    }

    pub(crate) fn add<N: NotificationMessage + 'static>(
        &self,
        handler: impl for<'a> Fn(N::Ref<'a>, NextNotification<N>) + 'static,
    ) {
        if let Some(chain) = self.chain.downcast_ref::<Pipeline<Handler<N>>>() {
            chain.add(Rc::new(handler));
        }
    }

    pub(crate) fn dispatch(&self) -> Dispatch {
        Rc::clone(&self.dispatch)
    }
}
