//! Tests definition requirements for outgoing values and incoming frames,
//! including provider-side rejection when client validation is disabled.

use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{ChannelOptions, IncomingStream, Requirement};
use duplex_test_support::conformance::calculator::{Count, Log, Progress, Quotient, Sum, User};
use duplex_test_support::future::ready_output;
use duplex_test_support::{Memory, recorder};

use crate::support::validation::{
    connected, divide, dividing, invalid, keeping_sums, progress, step, unvalidated,
};

#[test]
fn should_fail_a_message_whose_field_breaks_its_requirement_before_sending_it() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let handled = dividing(&calculator);
    let sent = user.send(&divide(13));

    queue.deliver();
    assert_eq!(sent.err(), Some(invalid(&[2], Requirement::Maximum)));
    assert_eq!(*handled.borrow(), 0);
}

#[test]
fn should_reject_a_field_breaking_its_requirement_that_the_client_didnt_validate() {
    let (user, calculator, queue) = connected(unvalidated(), ChannelOptions::default());
    let handled = dividing(&calculator);
    let quotient = user.send(&divide(13)).expect("the message is sent");

    queue.deliver();
    assert_eq!(
        ready_output(quotient),
        Err(invalid(&[2], Requirement::Maximum).with_failure())
    );
    assert_eq!(*handled.borrow(), 0);
}

#[test]
fn should_accept_what_meets_its_requirements() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    dividing(&calculator);

    let quotient = user.send(&divide(12)).expect("the message is sent");

    queue.deliver();
    assert_eq!(ready_output(quotient), Ok(Quotient::Exact { value: 2 }));
}

#[test]
fn should_reject_a_number_past_its_bounds() {
    let (user, calculator, queue) = connected(unvalidated(), ChannelOptions::default());

    calculator.handle(|_: Count, _responder, _next| {});

    let mut stream = user
        .send(&Count { to: 10_001 })
        .expect("the message is sent");

    queue.deliver();
    assert_eq!(
        ready_output(stream.start()),
        Err(invalid(&[0], Requirement::Maximum))
    );
}

#[test]
fn should_name_where_a_nested_value_breaks_its_requirement() {
    let (_user, calculator, queue) = Memory::connected_pair::<User>();

    queue.deliver();

    let errors = [
        calculator.send(&progress(vec![step("divide"), step("Add")])),
        calculator.send(&progress(vec![step("")])),
        calculator.send(&progress(vec![step(&"a".repeat(33))])),
    ];

    assert_eq!(
        errors.map(Result::err),
        [
            Some(invalid(&[2, 1, 0], Requirement::Pattern)),
            Some(invalid(&[2, 0, 0], Requirement::MinLength)),
            Some(invalid(&[2, 0, 0], Requirement::MaxLength)),
        ]
    );
}

#[test]
fn should_leave_what_the_provider_sends_unvalidated_by_the_client() {
    let (user, calculator, queue) = connected(ChannelOptions::default(), unvalidated());
    let (received, record) = recorder();

    user.on::<Progress>(move |progress, _next| record(progress.steps[0].name.to_string()));
    calculator
        .send(&progress(vec![step("Not Validated")]))
        .expect("the message is sent");
    queue.deliver();
    assert_eq!(*received.borrow(), ["Not Validated"]);
}

#[test]
fn should_fail_a_message_past_its_limit_before_sending_it() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let logged = Rc::new(RefCell::new(0));

    calculator.on::<Log>({
        let logged = Rc::clone(&logged);

        move |_, _next| *logged.borrow_mut() += 1
    });

    let text = "x".repeat(4 * 1024 * 1024);
    let sent = user.send(&Log { text: text.into() });

    queue.deliver();
    assert_eq!(sent, Err(invalid(&[], Requirement::Limit)));
    assert_eq!(*logged.borrow(), 0);
}

#[test]
fn should_drop_a_notification_past_its_limit_that_the_client_didnt_validate() {
    let (user, calculator, queue) = connected(unvalidated(), ChannelOptions::default());
    let (logged, record) = recorder();

    calculator.on::<Log>(move |log, _next| record(log.text.len()));
    user.send(&Log {
        text: "x".repeat(4 * 1024 * 1024).into(),
    })
    .expect("the message is sent");
    user.send(&Log {
        text: "fits".into(),
    })
    .expect("the message is sent");
    queue.deliver();
    assert_eq!(*logged.borrow(), [4]);
}

#[test]
fn should_reject_a_stream_item_past_its_bounds() {
    let (user, calculator, queue) = connected(unvalidated(), ChannelOptions::default());
    let reading: Rc<RefCell<Vec<IncomingStream<(), i64>>>> = Rc::default();
    let responders = Rc::new(RefCell::new(Vec::new()));

    calculator.handle({
        let (reading, responders) = (Rc::clone(&reading), Rc::clone(&responders));

        move |_: Sum, (terms, responder), _next| {
            reading.borrow_mut().push(terms);
            responders.borrow_mut().push(responder);
        }
    });

    let (mut terms, sum) = user.send(&Sum {}).expect("the message is sent");

    queue.deliver();
    ready_output(terms.send(&1_000_001)).expect("the term is sent");
    queue.deliver();

    let read = ready_output(reading.borrow_mut()[0].next());

    queue.deliver();
    assert_eq!(read, Some(Err(invalid(&[], Requirement::Maximum))));
    assert_eq!(ready_output(sum), Err(invalid(&[], Requirement::Maximum)));
}

#[test]
fn should_reject_a_stream_past_the_most_items_it_has() {
    let window = ChannelOptions {
        stream_window: Some(2000),
        ..unvalidated()
    };
    let (user, calculator, queue) = connected(window, window);

    keeping_sums(&calculator);

    let (mut terms, sum) = user.send(&Sum {}).expect("the message is sent");

    queue.deliver();
    for _ in 0..1001 {
        ready_output(terms.send(&1)).expect("the term is sent");
    }

    queue.deliver();
    assert_eq!(ready_output(sum), Err(invalid(&[], Requirement::MaxLength)));
}

#[test]
fn should_fail_an_item_past_the_most_a_stream_has_before_sending_it() {
    let window = ChannelOptions {
        stream_window: Some(2000),
        ..ChannelOptions::default()
    };
    let (user, calculator, queue) = connected(window, window);

    keeping_sums(&calculator);

    let (mut terms, _sum) = user.send(&Sum {}).expect("the message is sent");

    queue.deliver();
    for _ in 0..1000 {
        ready_output(terms.send(&1)).expect("the term is sent");
    }

    let past = ready_output(terms.send(&1));

    assert_eq!(past, Err(invalid(&[], Requirement::MaxLength)));
}
