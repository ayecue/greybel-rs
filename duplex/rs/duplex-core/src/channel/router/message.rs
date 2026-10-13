use std::any::Any;
use std::rc::Rc;

use super::MessageDispatch as Dispatch;
use super::pipeline::{HandlerContainer, Pipeline};
use crate::binary::ValidatedDecode;
use crate::definition::ErrorCode;
use crate::driver::Driver;
use crate::frame::Rejection;
use crate::protocol::{Exchange, Message, Peer};

type Responder<M> = <<M as Message>::Exchange as Exchange>::Responder;
type Handler<M> = dyn Fn(M, Responder<M>, NextMessage<M>);

/// Remaining message handlers in the chain.
pub struct NextMessage<M: Message> {
    handlers: HandlerContainer<Handler<M>>,
    index: usize,
}

impl<M: Message> NextMessage<M> {
    fn first(handlers: HandlerContainer<Handler<M>>) -> Self {
        Self { handlers, index: 0 }
    }

    /// Invokes the next handler or rejects with `UnhandledMessage` when none remain.
    pub fn call(self, message: M, responder: Responder<M>) {
        let Some(handler) = self.handlers.get(self.index).cloned() else {
            return M::Exchange::reject(responder, ErrorCode::UnhandledMessage);
        };
        let next = Self {
            handlers: self.handlers,
            index: self.index + 1,
        };

        handler(message, responder, next);
    }
}

pub(crate) struct MessageHandler<P: Peer, D: Driver> {
    dispatch: Dispatch<P, D>,
    chain: Rc<dyn Any>,
    meta: MessageHandlerMeta,
    refs: u8,
}

/// Whether the handled message has an item stream.
#[derive(Clone, Copy)]
pub(crate) enum Streaming {
    No,
    /// Declared streams contain items of at most `max_size` bytes.
    Items {
        max_size: Option<u32>,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct MessageHandlerMeta {
    pub(crate) streaming: Streaming,
    pub(crate) has_response: bool,
}

impl<P: Peer, D: Driver> MessageHandler<P, D> {
    pub(crate) fn new<M>() -> Self
    where
        M: Message + ValidatedDecode + 'static,
    {
        let chain: Rc<Pipeline<Handler<M>>> = Rc::default();
        let route = Rc::clone(&chain);

        Self {
            dispatch: Rc::new(move |channel, id, body, reads| match body.decode::<M>() {
                Ok(message) => {
                    let exchange = channel.start_exchange(id, reads);
                    let responder = M::Exchange::responder(&exchange);

                    NextMessage::first(route.handlers()).call(message, responder);
                }
                Err(error) => channel.handle_message_rejection(
                    id,
                    Rejection::of(&error),
                    M::Exchange::RESPONDER_REFS,
                ),
            }),
            chain,
            meta: MessageHandlerMeta {
                streaming: if M::Exchange::STREAMS {
                    Streaming::Items {
                        max_size: M::REQUIREMENTS.message_item_size,
                    }
                } else {
                    Streaming::No
                },
                has_response: M::Exchange::STREAMS_RESPONSE,
            },
            refs: M::Exchange::RESPONDER_REFS,
        }
    }

    pub(crate) fn add<M: Message + 'static>(
        &self,
        handler: impl Fn(M, Responder<M>, NextMessage<M>) + 'static,
    ) {
        if let Some(chain) = self.chain.downcast_ref::<Pipeline<Handler<M>>>() {
            chain.add(Rc::new(handler));
        }
    }

    pub(crate) fn meta(&self) -> MessageHandlerMeta {
        self.meta
    }

    pub(crate) fn dispatch(&self) -> Dispatch<P, D> {
        Rc::clone(&self.dispatch)
    }

    pub(crate) fn refs(&self) -> u8 {
        self.refs
    }
}
