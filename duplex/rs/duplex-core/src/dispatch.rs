//! Generic dispatch installed by generated protocol routers.

use std::rc::Rc;

use crate::{
    Exchange, Message, NextMessage, NextNotification, NotificationMessage, ValidatedDecode,
};

/// Responder associated with message `M`.
pub type ResponderOf<M> = <<M as Message>::Exchange as Exchange>::Responder;

/// Handles every inbound message of a generated protocol, forwarding unrouted messages to `next`.
pub trait Dispatch: 'static {
    /// Handles a message with a typed responder.
    fn message<M: Message + ValidatedDecode + 'static>(
        self: Rc<Self>,
        message: M,
        responder: ResponderOf<M>,
        next: NextMessage<M>,
    );

    /// Handles a notification borrowed from its input frame.
    fn notification<N: NotificationMessage + 'static>(
        &self,
        message: N::Ref<'_>,
        next: NextNotification<N>,
    );
}
