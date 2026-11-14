use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use duplex_core::{ChannelOptions, Error, ErrorCode, Health, HealthCheck, Role};
use duplex_test_support::binary::bytes;
use duplex_test_support::conformance::Adder;
use duplex_test_support::conformance::calculator::{
    Add, CalculatorRouter, Divide, DivisionFailure, Phase, Progress, Quotient, Step, User,
    UserChannel,
};
use duplex_test_support::frames::Frame;
use duplex_test_support::future::{poll, ready_output};
use duplex_test_support::{Memory, recorder};

use crate::support::{
    CONNECTION, Logger, add_request, batch, healths, keep_add_requests, log, logs,
    notifications_then, post_frames,
};

#[test]
fn should_wait_for_the_task_to_end_before_posting() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();

    user.send(&log("adding"))
        .expect("the log notification is sent");
    assert_eq!(queue.take_batches(), Vec::<Vec<u8>>::new());
}

#[test]
fn should_post_a_full_batch_at_once() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();

    user.send(&log(&"x".repeat(70_000)))
        .expect("the log notification is sent");
    assert_eq!(queue.take_batches().len(), 1);
}

#[test]
fn should_carry_a_message_past_a_batch_in_batches_of_a_batch_at_most() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let logged = logs(&calculator);
    let text = "x".repeat(1 << 20);

    user.send(&log(&text))
        .expect("the log notification is sent");

    let posted = queue.deliver_bytes();

    assert!(posted.len() > 16);
    assert!(posted.iter().all(|batch| batch.len() <= 64 * 1024 + 16));
    assert_eq!(*logged.borrow(), [text]);
}

#[test]
fn should_fail_a_message_past_the_most_a_message_takes_and_send_nothing() {
    let options = ChannelOptions {
        max_message: Some(1024),
        ..ChannelOptions::default()
    };
    let (user, _calculator, queue) = Memory::connected_pair_with_options::<User>(options);

    let sent = user.send(&log(&"x".repeat(2048)));

    assert_eq!(sent.err(), Some(Error::Rejection(ErrorCode::Overload)));
    assert_eq!(queue.take_batches(), Vec::<Vec<u8>>::new());
}

#[test]
fn should_resolve_a_reply_once_the_peer_replies() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let pending = keep_add_requests(&calculator);
    let mut reply = user.send(&Add { a: 2, b: 2 }).expect("the request is sent");

    queue.deliver();

    let _ = poll(&mut reply);
    let (add, response) = pending
        .borrow_mut()
        .take()
        .expect("the calculator received the add request");

    response.respond(add.a * add.b);
    queue.deliver();
    assert_eq!(ready_output(reply), Ok(4));
}

#[test]
fn should_fail_failed_and_unresponded_requests() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    calculator.handle(|_: Add, response, _next| drop(response));
    calculator.handle(|_: Divide, response, _next| response.fail(DivisionFailure::ByZero));

    let unresponded = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");
    let failed = user
        .send(&Divide {
            dividend: 1,
            divisor: 0,
            precision: None,
        })
        .expect("the request is sent");

    queue.deliver();
    assert_eq!(
        ready_output(failed),
        Err(Error::Failure(DivisionFailure::ByZero))
    );
    assert_eq!(
        ready_output(unresponded),
        Err(Error::Rejection(ErrorCode::DroppedMessage))
    );
}

#[test]
fn should_tell_a_removed_health_listener_no_more_changes() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let (removed, record) = recorder();
    let id = user.on_health(record);
    let kept = healths(&user);

    user.remove_hook(id);
    for _ in 0..3 {
        user.check();
    }

    queue.deliver();
    assert!(removed.borrow().is_empty());
    assert_eq!(*kept.borrow(), [Health::Unhealthy, Health::Healthy]);
}

#[test]
fn should_post_at_once_on_close() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();

    user.close(ErrorCode::ClosedChannel);

    let posted = queue.take_batches();

    assert_eq!(posted.len(), 1);
}

#[test]
fn should_fail_pending_requests_when_the_client_drops() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let reply = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    drop(user);
    assert_eq!(
        ready_output(reply),
        Err(Error::ChannelEnd(ErrorCode::ClosedChannel))
    );
    assert_eq!(
        queue.queued_slices(),
        [batch(
            Role::Client,
            CONNECTION,
            vec![
                add_request(0, 1, 2),
                Frame::Close {
                    reason: u32::from(ErrorCode::ClosedChannel)
                }
            ]
        )]
    );
}

#[test]
fn should_resolve_closed_with_the_reason_once_closed() {
    let (user, _calculator, _queue) = Memory::connected_pair::<User>();
    let mut closed = user.close_event();
    let waiting = poll(&mut closed);

    user.close(ErrorCode::Panic);
    assert!(waiting.is_pending());
    assert_eq!(ready_output(closed), ErrorCode::Panic);
    assert_eq!(ready_output(user.close_event()), ErrorCode::Panic);
}

#[test]
fn should_run_the_health_check_on_a_timer_of_the_driver() {
    let without = ChannelOptions::without_health_checks();
    let (_user, _calculator, checked) = Memory::pair::<User>();
    let (_user, _calculator, unchecked) = Memory::pair_with_options::<User>(without);

    checked.run_tasks();
    unchecked.run_tasks();

    let interval = HealthCheck::default().interval;

    assert_eq!(checked.timers(), [interval, interval]);
    assert_eq!(unchecked.timers(), []);
}

#[test]
fn should_end_the_providers_connection_when_the_client_drops() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let _reply = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    drop(user);
    queue.deliver();
    assert_eq!(calculator.connection(), None);
}

#[test]
fn should_drop_a_response_for_an_ended_connection() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let pending = keep_add_requests(&calculator);
    let _reply = user.send(&Add { a: 2, b: 2 }).expect("the request is sent");

    queue.deliver();
    user.close(ErrorCode::ClosedChannel);
    queue.deliver();

    let (add, response) = pending
        .borrow_mut()
        .take()
        .expect("the calculator received the add request");

    response.respond(add.a + add.b);
    assert_eq!(queue.queued_batches(), Vec::<Vec<u8>>::new());
}

#[test]
fn should_send_through_the_peers_channel_trait_without_the_drivers_type() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let channel: Rc<dyn UserChannel> = Rc::new(user);

    Adder.attach(&calculator);

    let added = channel
        .send_add(&Add { a: 2, b: 3 })
        .expect("the request is sent");

    queue.deliver();
    assert_eq!(ready_output(added), Ok(5));
}

#[test]
fn should_run_routers_in_order_passing_on_what_they_do_not_handle() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let logged = Rc::new(RefCell::new(Vec::new()));

    Logger(Rc::clone(&logged)).attach(&calculator);
    Adder.attach(&calculator);
    user.send(&log("adding"))
        .expect("the log notification is sent");

    let added = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");
    let divided = user
        .send(&Divide {
            dividend: 1,
            divisor: 1,
            precision: None,
        })
        .expect("the request is sent");

    queue.deliver();
    assert_eq!(*logged.borrow(), ["adding"]);
    assert_eq!(ready_output(added), Ok(3));
    assert_eq!(
        ready_output(divided),
        Err(Error::Rejection(ErrorCode::UnhandledMessage))
    );
}

#[test]
fn should_drop_a_held_request_whose_reply_is_dropped() {
    let (user, calculator, queue) = Memory::pair::<User>();
    let pending = keep_add_requests(&calculator);
    let reply = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    drop(reply);
    queue.deliver();
    assert!(pending.borrow().is_none());
}

#[test]
fn should_take_a_burst_of_messages_its_default_limits_allow() {
    const BURST: i64 = 8192;
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let (waiting, record) = recorder();

    calculator.handle(move |add: Add, response, _next| record((add, response)));

    let sums: Vec<_> = (0..BURST)
        .map(|a| user.send(&Add { a, b: 1 }).expect("the request is sent"))
        .collect();

    queue.deliver();
    assert_eq!(waiting.borrow().len(), 8192);
    for (add, response) in waiting.borrow_mut().drain(..) {
        response.respond(add.a + add.b);
    }

    queue.deliver();

    let sums: Vec<_> = sums.into_iter().map(ready_output).collect();

    assert_eq!(sums, (1..=BURST).map(Ok).collect::<Vec<_>>());
}

#[test]
fn should_hand_a_notification_to_its_handlers_borrowing_what_it_came_in() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    queue.deliver();

    let (received, record) = recorder();

    user.on::<Progress>(move |progress, next| {
        record(progress.clone().into_owned());
        next.call(progress);
    });

    let sent = Progress {
        phase: Phase::Active,
        done_share: Some(0.5),
        steps: vec![Step {
            name: "divide".to_owned(),
            result: Some(Quotient::WithRemainder {
                value: 2,
                remainder: 1,
            }),
            duration: Some(3),
        }]
        .into(),
        counts: [("lines".to_owned(), 3)].into_iter().collect(),
        note: Some(Some("almost".into())),
        eta: Some(2.5),
    };

    calculator
        .send(&sent)
        .expect("the progress notification is sent");
    queue.deliver();
    assert_eq!(*received.borrow(), [sent]);
}

#[test]
fn should_take_a_burst_past_what_it_responds_to_at_once_when_its_handlers_keep_up() {
    const BURST: i64 = 10_000;
    let (user, calculator, queue) = Memory::connected_pair::<User>();

    Adder.attach(&calculator);

    let sums: Vec<_> = (0..BURST)
        .map(|a| user.send(&Add { a, b: 1 }).expect("the request is sent"))
        .collect();

    queue.deliver();

    let sums: Vec<_> = sums.into_iter().map(ready_output).collect();

    assert_eq!(sums, (1..=BURST).map(Ok).collect::<Vec<_>>());
}

#[test]
fn should_deliver_a_response_after_two_thousand_earlier_notifications() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let (received, record) = recorder();

    user.on::<Progress>(move |_, _next| record(()));

    let mut response = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    queue.take();
    post_frames(
        &queue,
        notifications_then(
            2000,
            Frame::ResponseEnd {
                id: 0,
                body: bytes(&3_i64),
            },
        ),
    );
    queue.step();

    assert!(poll(&mut response).is_pending());
    assert_eq!(received.borrow().len(), 0);
    queue.run_tasks();
    assert_eq!(received.borrow().len(), 2000);
    assert_eq!(ready_output(response), Ok(3));
}

#[test]
fn should_keep_response_order_when_notification_handlers_exceed_a_cycle() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let (received, record) = recorder();

    user.on::<Progress>({
        let queue = queue.clone();

        move |_, _next| {
            queue.advance(Duration::from_millis(2));
            record(());
        }
    });

    let mut response = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    queue.take();
    post_frames(
        &queue,
        notifications_then(
            2000,
            Frame::ResponseEnd {
                id: 0,
                body: bytes(&3_i64),
            },
        ),
    );
    queue.step();
    queue.run_task();

    assert_eq!(received.borrow().len(), 64);
    assert!(poll(&mut response).is_pending());
    queue.run_tasks();
    assert_eq!(received.borrow().len(), 2000);
    assert_eq!(ready_output(response), Ok(3));
}

#[test]
fn should_deliver_an_unqueued_response_in_the_same_read() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let response = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    queue.take();
    post_frames(
        &queue,
        vec![Frame::ResponseEnd {
            id: 0,
            body: bytes(&3_i64),
        }],
    );
    queue.step();

    assert_eq!(ready_output(response), Ok(3));
}

#[test]
fn should_neither_time_out_nor_cancel_a_received_response_waiting_for_earlier_work() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let (received, record) = recorder();

    user.on::<Progress>(move |_, _next| record(()));

    let mut response = user.send(&Add { a: 1, b: 2 }).expect("the request is sent");

    queue.take();
    post_frames(
        &queue,
        notifications_then(
            1025,
            Frame::ResponseEnd {
                id: 0,
                body: bytes(&3_i64),
            },
        ),
    );
    queue.step();
    assert!(poll(&mut response).is_pending());
    queue.advance(Duration::from_secs(2));
    assert!(poll(&mut response).is_pending());

    drop(response);
    queue.run_tasks();

    assert_eq!(received.borrow().len(), 1025);
    assert!(
        queue
            .queued_slices()
            .iter()
            .flat_map(|slice| &slice.frames)
            .all(|frame| !matches!(frame, Frame::Cancel { .. }))
    );
}
