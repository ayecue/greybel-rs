mod message;
mod notification;
mod pipeline;

use std::cell::RefCell;
use std::rc::Rc;

pub use message::NextMessage;
pub(crate) use message::{MessageHandler, MessageHandlerMeta, Streaming};
pub use notification::NextNotification;
pub(crate) use notification::NotificationHandler;

use super::connection::PendingConnection;
use super::message::Body;
use super::state::ChannelState;
use crate::binary::ValidatedDecode;
use crate::driver::Driver;
use crate::keys::EntryId;
use crate::protocol::{Exchange, Message, NotificationMessage, Peer, Protocol};

pub(crate) type NotificationDispatch = Rc<dyn Fn(&Body)>;
pub(crate) type MessageDispatch<P, D> = Rc<dyn Fn(&ChannelState<P, D>, EntryId, &Body, bool)>;
pub(crate) type ConnectDispatch<Pr> = Rc<dyn Fn(<Pr as Protocol>::Open, PendingConnection<Pr>)>;

pub(crate) struct HandlerTable<P: Peer, D: Driver> {
    notifications: RefCell<Vec<Option<NotificationHandler>>>,
    messages: RefCell<Vec<Option<MessageHandler<P, D>>>>,
    connect: RefCell<Option<ConnectDispatch<P::Protocol>>>,
}

impl<P: Peer, D: Driver> Default for HandlerTable<P, D> {
    fn default() -> Self {
        Self {
            notifications: RefCell::default(),
            messages: RefCell::default(),
            connect: RefCell::default(),
        }
    }
}

type Responder<M> = <<M as Message>::Exchange as Exchange>::Responder;

impl<P: Peer, D: Driver> HandlerTable<P, D> {
    pub(crate) fn on<N>(&self, handler: impl for<'a> Fn(N::Ref<'a>, NextNotification<N>) + 'static)
    where
        N: NotificationMessage<Sender = P::Remote> + 'static,
    {
        Self::slot(&mut self.notifications.borrow_mut(), N::ID)
            .get_or_insert_with(NotificationHandler::new::<N>)
            .add::<N>(handler);
    }

    pub(crate) fn handle<M>(&self, handler: impl Fn(M, Responder<M>, NextMessage<M>) + 'static)
    where
        M: Message<Sender = P::Remote> + ValidatedDecode + 'static,
    {
        Self::slot(&mut self.messages.borrow_mut(), M::ID)
            .get_or_insert_with(MessageHandler::new::<M>)
            .add::<M>(handler);
    }

    pub(crate) fn on_connect(&self, handler: ConnectDispatch<P::Protocol>) {
        self.connect.replace(Some(handler));
    }

    pub(crate) fn connect(&self) -> Option<ConnectDispatch<P::Protocol>> {
        self.connect.borrow().clone()
    }

    pub(crate) fn handles_notification(&self, type_id: u32) -> bool {
        self.notifications
            .borrow()
            .get(type_id as usize)
            .is_some_and(Option::is_some)
    }

    pub(crate) fn notification(&self, type_id: u32) -> Option<NotificationDispatch> {
        self.notifications
            .borrow()
            .get(type_id as usize)
            .and_then(Option::as_ref)
            .map(NotificationHandler::dispatch)
    }

    pub(crate) fn message(&self, type_id: u32) -> Option<MessageHandlerMeta> {
        self.messages
            .borrow()
            .get(type_id as usize)
            .and_then(Option::as_ref)
            .map(MessageHandler::meta)
    }

    /// Returns dispatcher and required reference count for message type ID `type_id`.
    pub(crate) fn message_dispatch(&self, type_id: u32) -> Option<(MessageDispatch<P, D>, u8)> {
        self.messages
            .borrow()
            .get(type_id as usize)
            .and_then(Option::as_ref)
            .map(|route| (route.dispatch(), route.refs()))
    }

    fn slot<R>(routes: &mut Vec<Option<R>>, id: u32) -> &mut Option<R> {
        let index = id as usize;

        if routes.len() <= index {
            routes.resize_with(index + 1, || None);
        }

        &mut routes[index]
    }
}
