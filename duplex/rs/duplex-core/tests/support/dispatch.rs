use std::any::Any;
use std::rc::Rc;

use duplex_core::{Dispatch, NextMessage, NextNotification, Responder};
use duplex_test_support::conformance::calculator::Add;

pub(crate) struct Sum;

impl Dispatch for Sum {
    fn message<M: duplex_core::Message + duplex_core::ValidatedDecode + 'static>(
        self: Rc<Self>,
        message: M,
        responder: duplex_core::ResponderOf<M>,
        _next: NextMessage<M>,
    ) {
        let message = &message as &dyn Any;

        if let Some(add) = message.downcast_ref::<Add>() {
            let responder = (Box::new(responder) as Box<dyn Any>)
                .downcast::<Responder<i64>>()
                .expect("Add has an i64 responder");

            responder.respond(add.a + add.b);
        } else {
            // This generic test dispatcher handles only Add.
            unreachable!("unexpected message")
        }
    }

    fn notification<N: duplex_core::NotificationMessage + 'static>(
        &self,
        message: N::Ref<'_>,
        next: NextNotification<N>,
    ) {
        next.call(message);
    }
}

pub(crate) struct Forward;

impl Dispatch for Forward {
    fn message<M: duplex_core::Message + duplex_core::ValidatedDecode + 'static>(
        self: Rc<Self>,
        message: M,
        responder: duplex_core::ResponderOf<M>,
        next: NextMessage<M>,
    ) {
        next.call(message, responder);
    }

    fn notification<N: duplex_core::NotificationMessage + 'static>(
        &self,
        message: N::Ref<'_>,
        next: NextNotification<N>,
    ) {
        next.call(message);
    }
}
