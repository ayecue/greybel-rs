use std::rc::Rc;

use duplex_test_support::conformance::calculator::{Add, Log, LogRef, User, attach_dispatch};
use duplex_test_support::future::ready_output;
use duplex_test_support::{Memory, recorder};

use super::support::dispatch::{Forward, Sum};

#[test]
fn should_dispatch_a_generated_message_to_one_generic_handler() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    attach_dispatch::<_, Sum>(&calculator, Rc::new(Sum));

    let reply = user.send(&Add { a: 20, b: 22 }).unwrap();

    queue.deliver();

    assert_eq!(ready_output(reply), Ok(42));
}

#[test]
fn should_fall_through_to_the_existing_message_and_notification_handlers() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let (logged, record) = recorder();

    attach_dispatch::<_, Forward>(&calculator, Rc::new(Forward));
    calculator.handle(|message: Add, responder, _| responder.respond(message.a * message.b));
    calculator.on::<Log>(move |message: LogRef<'_>, _| record(message.text.to_owned()));

    let reply = user.send(&Add { a: 6, b: 7 }).unwrap();

    user.send(&Log {
        text: "forwarded".into(),
    })
    .unwrap();

    queue.deliver();

    assert_eq!(ready_output(reply), Ok(42));
    assert_eq!(*logged.borrow(), ["forwarded"]);
}
