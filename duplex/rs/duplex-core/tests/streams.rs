use std::cell::RefCell;
use std::pin::pin;
use std::rc::Rc;
use std::task::Poll;

use duplex_core::{Capabilities, Error, ErrorCode, Message as _, Role};
use duplex_test_support::binary::bytes;
use duplex_test_support::conformance::calculator::{
    Calculation, Count, Phase, Progress, Sum, User,
};
use duplex_test_support::frames::Frame;
use duplex_test_support::future::{poll, ready_output};
use duplex_test_support::{Memory, TaskQueue, recorder};

use crate::support::streams::{WINDOW, counting, pair, summing_later};
use crate::support::validation::progress;
use crate::support::{CONNECTION, batch};

#[test]
fn should_stream_a_response_its_start_items_and_end() {
    let (user, calculator, queue) = pair();
    let (tasks, _sent) = counting(&calculator);
    let mut stream = user
        .send(&Count { to: 3 })
        .expect("the stream request is sent");

    tasks.run(&queue);
    assert_eq!(ready_output(stream.start()), Ok(Phase::Active));

    let items: Vec<_> = (0..3).map(|_| ready_output(stream.next())).collect();

    assert_eq!(items, [Some(Ok(1)), Some(Ok(2)), Some(Ok(3))]);
    assert_eq!(ready_output(stream.end()), Ok(6));
}

#[test]
fn should_send_no_more_items_than_the_receiver_takes() {
    let (user, calculator, queue) = pair();
    let (tasks, sent) = counting(&calculator);
    let to = WINDOW * 4;
    let mut stream = user
        .send(&Count { to })
        .expect("the stream request is sent");

    tasks.run(&queue);

    let before = sent.borrow().len();
    let half = WINDOW / 2;

    for _ in 0..half {
        ready_output(stream.next());
    }

    tasks.run(&queue);
    assert_eq!(before, WINDOW as usize);
    assert_eq!(sent.borrow().len(), (WINDOW + half) as usize);
}

#[test]
fn should_stop_a_handler_whose_stream_is_dropped() {
    let (user, calculator, queue) = pair();
    let (tasks, sent) = counting(&calculator);
    let mut stream = user
        .send(&Count { to: WINDOW * 4 })
        .expect("the stream request is sent");

    tasks.run(&queue);
    ready_output(stream.next());
    drop(stream);
    tasks.run(&queue);
    assert_eq!(
        sent.borrow().last(),
        Some(&Err(Error::Rejection(ErrorCode::Cancellation)))
    );
}

#[test]
fn should_respond_to_what_a_message_streams_after_it() {
    let (user, calculator, queue) = pair();
    let (tasks, _sent) = counting(&calculator);
    let (mut terms, sum) = user.send(&Sum {}).expect("the stream request is sent");

    assert_eq!(ready_output(terms.send(&4)), Ok(()));
    tasks.run(&queue);
    for term in [5, 6] {
        assert_eq!(ready_output(terms.send(&term)), Ok(()));
    }

    ready_output(terms.finish()).expect("the outgoing stream is finished");
    tasks.run(&queue);
    assert_eq!(ready_output(sum), Ok(15));
}

#[test]
fn should_send_all_items_of_a_response_as_the_receiver_takes_them() {
    let (user, calculator, queue) = pair();
    let tasks = TaskQueue::default();

    calculator.handle({
        let tasks = tasks.clone();

        move |count: Count, responder, _next| {
            tasks.spawn(async move {
                let mut items = responder.start(Phase::Active);

                if items.send_all(1..=count.to).await.is_ok() {
                    items.end((1..=u64::from(count.to)).sum());
                }
            });
        }
    });

    let to = WINDOW * 3;

    let mut stream = user
        .send(&Count { to })
        .expect("the stream request is sent");
    let mut items = Vec::new();

    while items.len() < to as usize {
        tasks.run(&queue);
        while let Poll::Ready(Some(item)) = poll(&mut pin!(stream.next())) {
            items.push(item.expect("each response item is readable"));
        }
    }

    tasks.run(&queue);

    assert_eq!(items, (1..=to).collect::<Vec<_>>());
    assert_eq!(ready_output(stream.end()), Ok((1..=u64::from(to)).sum()));
}

#[test]
fn should_send_all_items_a_message_streams_after_it() {
    let (user, calculator, queue) = pair();
    let (tasks, _sent) = counting(&calculator);
    let count = i64::from(WINDOW * 3);
    let (mut terms, sum) = user.send(&Sum {}).expect("the stream request is sent");

    tasks.spawn(async move {
        terms
            .send_all(vec![
                1;
                usize::try_from(count).expect("a few windows of items")
            ])
            .await
            .expect("every term is sent");
        terms
            .finish()
            .await
            .expect("the outgoing stream is finished");
    });
    tasks.run(&queue);

    assert_eq!(ready_output(sum), Ok(count));
}

#[test]
fn should_wait_for_credit_before_the_receiver_takes_more() {
    let (user, calculator, queue) = pair();
    let (tasks, _sent) = counting(&calculator);
    let (mut terms, sum) = user.send(&Sum {}).expect("the stream request is sent");

    assert_eq!(ready_output(terms.send(&1)), Ok(()));

    let waits_for_its_receiver = poll(&mut pin!(terms.send(&1))).is_pending();

    tasks.run(&queue);
    for _ in 1..WINDOW {
        assert_eq!(ready_output(terms.send(&1)), Ok(()));
    }

    let waits = poll(&mut pin!(terms.send(&1))).is_pending();

    tasks.run(&queue);

    let goes_on = poll(&mut pin!(terms.send(&1))).is_ready();

    ready_output(terms.finish()).expect("the outgoing stream is finished");
    tasks.run(&queue);
    assert!(waits_for_its_receiver);
    assert!(waits);
    assert!(goes_on);
    assert_eq!(ready_output(sum), Ok(i64::from(WINDOW) + 1));
}

#[test]
fn should_fail_a_stream_whose_connection_ends() {
    let (user, calculator, queue) = pair();
    let (tasks, _sent) = counting(&calculator);
    let mut stream = user
        .send(&Count { to: WINDOW * 4 })
        .expect("the stream request is sent");

    tasks.run(&queue);
    calculator.close(ErrorCode::ClosedChannel);
    tasks.run(&queue);
    assert_eq!(ready_output(stream.start()), Ok(Phase::Active));

    let items: Vec<_> = std::iter::from_fn(|| ready_output(stream.next())).collect();

    assert_eq!(
        items.last(),
        Some(&Err(Error::ChannelEnd(ErrorCode::ClosedChannel)))
    );
    assert_eq!(items.len(), WINDOW as usize + 1);
}

#[test]
fn should_support_a_protocol_whose_client_handles_messages_over_persistent_transports_only() {
    let requires = <Calculation as duplex_core::Protocol>::REQUIRES;
    let one_shot = Capabilities::DUPLEX.without(Capabilities::PERSISTENT);
    let supported = [Capabilities::DUPLEX, one_shot]
        .map(|capabilities| capabilities.supports(Role::Client, requires));

    assert_eq!(requires, Capabilities::PERSISTENT);
    assert_eq!(supported, [true, false]);
}

#[test]
fn should_fail_the_response_of_a_message_whose_stream_is_dropped() {
    let (user, calculator, queue) = pair();
    let _kept = summing_later(&calculator);
    let (terms, sum) = user.send(&Sum {}).expect("the stream request is sent");

    queue.deliver();
    drop(terms);
    assert_eq!(
        ready_output(sum),
        Err(Error::Rejection(ErrorCode::Cancellation))
    );
}

#[test]
fn should_fail_the_response_of_a_queued_message_whose_stream_is_dropped() {
    let (user, calculator, queue) = Memory::pair::<User>();
    let kept = summing_later(&calculator);
    let (terms, sum) = user.send(&Sum {}).expect("the stream request is sent");

    drop(terms);
    queue.deliver();
    assert_eq!(
        ready_output(sum),
        Err(Error::Rejection(ErrorCode::Cancellation))
    );
    assert!(kept.borrow().is_empty());
}

#[test]
fn should_leave_a_newer_message_alone_when_the_stream_of_a_settled_one_is_dropped() {
    let (user, calculator, queue) = pair();
    let kept = summing_later(&calculator);
    let (mut first_terms, first) = user.send(&Sum {}).expect("the stream request is sent");

    queue.deliver();

    let (_, responder) = kept.borrow_mut().remove(0);

    responder.respond(1);
    queue.deliver();
    assert_eq!(ready_output(first), Ok(1));

    let (_second_terms, second) = user.send(&Sum {}).expect("the stream request is sent");

    queue.deliver();

    let wrote = ready_output(first_terms.send(&5));

    drop(first_terms);
    queue.deliver();

    let (mut second_received, responder) = kept.borrow_mut().remove(0);

    assert!(wrote.is_err());
    assert!(!responder.is_finished());
    assert!(poll(&mut pin!(second_received.next())).is_pending());
    responder.respond(2);
    queue.deliver();
    assert_eq!(ready_output(second), Ok(2));
}

#[test]
fn should_ask_nothing_of_a_newer_response_while_reading_a_settled_one() {
    let (user, calculator, queue) = pair();
    let (tasks, sent) = counting(&calculator);
    let mut first = user
        .send(&Count { to: WINDOW })
        .expect("the stream request is sent");

    tasks.run(&queue);

    let _second = user
        .send(&Count { to: WINDOW * 2 })
        .expect("the stream request is sent");

    tasks.run(&queue);

    let before = sent.borrow().len();

    assert_eq!(ready_output(first.start()), Ok(Phase::Active));
    while ready_output(first.next()).is_some() {}

    tasks.run(&queue);
    assert_eq!(before, WINDOW as usize * 2);
    assert_eq!(sent.borrow().len(), before);
}

#[test]
fn should_grant_nothing_to_a_newer_stream_while_reading_a_settled_one() {
    let (user, calculator, queue) = pair();
    let kept = summing_later(&calculator);
    let (mut first_terms, first) = user.send(&Sum {}).expect("the stream request is sent");

    assert_eq!(ready_output(first_terms.send(&1)), Ok(()));
    queue.deliver();
    for _ in 1..WINDOW {
        assert_eq!(ready_output(first_terms.send(&1)), Ok(()));
    }

    queue.deliver();

    let (mut first_received, responder) = kept.borrow_mut().remove(0);

    responder.respond(0);
    queue.deliver();
    assert_eq!(ready_output(first), Ok(0));

    let (mut second_terms, _second) = user.send(&Sum {}).expect("the stream request is sent");

    assert_eq!(ready_output(second_terms.send(&1)), Ok(()));
    queue.deliver();
    for _ in 1..WINDOW {
        assert_eq!(ready_output(second_terms.send(&1)), Ok(()));
    }

    queue.deliver();
    for _ in 0..WINDOW {
        assert_eq!(ready_output(first_received.next()), Some(Ok(1)));
    }

    queue.deliver();
    assert!(poll(&mut pin!(second_terms.send(&1))).is_pending());
}

#[test]
fn should_reject_a_message_an_item_of_whose_stream_cannot_be_read() {
    let (user, calculator, queue) = pair();
    let (tasks, _sent) = counting(&calculator);
    let (_terms, sum) = user.send(&Sum {}).expect("the stream request is sent");

    tasks.run(&queue);

    let unreadable = batch(
        Role::Client,
        CONNECTION,
        vec![Frame::MessageItem {
            id: 0,
            body: vec![],
        }],
    );

    queue.post(Role::Provider, unreadable.encode());
    tasks.run(&queue);
    assert_eq!(
        ready_output(sum),
        Err(Error::Rejection(ErrorCode::MalformedMessage))
    );
}

#[test]
fn should_end_the_stream_of_a_queued_message_once_it_is_sent() {
    let (user, calculator, queue) = Memory::pair::<User>();
    let (tasks, _sent) = counting(&calculator);
    let (terms, sum) = user.send(&Sum {}).expect("the stream request is sent");
    let ended = Rc::new(RefCell::new(None));

    tasks.spawn({
        let ended = Rc::clone(&ended);

        async move {
            *ended.borrow_mut() = Some(terms.finish().await);
        }
    });
    tasks.run(&queue);
    assert_eq!(*ended.borrow(), Some(Ok(())));
    assert_eq!(ready_output(sum), Ok(0));
}

#[test]
fn should_reject_a_message_whose_handler_takes_longer_than_its_timeout() {
    let (user, calculator, queue) = pair();
    let (kept, record) = recorder();

    calculator.handle(move |_: Count, responder, _next| record(responder));

    let mut stream = user
        .send(&Count { to: 3 })
        .expect("the stream request is sent");

    queue.deliver();

    queue.advance(std::time::Duration::from_secs(1));
    queue.deliver();

    assert_eq!(
        ready_output(stream.start()),
        Err(Error::Rejection(ErrorCode::Timeout))
    );
    assert!(kept.borrow()[0].is_finished());
}

#[test]
fn should_deliver_response_stream_items_after_earlier_notifications() {
    let (user, _calculator, queue) = pair();
    let (received, record) = recorder();

    user.on::<Progress>(move |_, _next| record(()));

    let mut stream = user
        .send(&Count { to: 1 })
        .expect("the stream request is sent");

    queue.take();

    let notification = progress(Vec::new());
    let mut frames: Vec<_> = (0..1025)
        .map(|_| Frame::Message {
            type_id: Progress::ID,
            id: None,
            body: bytes(&notification),
        })
        .collect();

    frames.extend([
        Frame::ResponseStart {
            id: 0,
            body: bytes(&Phase::Active),
        },
        Frame::ResponseItem {
            id: 0,
            body: bytes(&1_u64),
        },
        Frame::ResponseEnd {
            id: 0,
            body: bytes(&1_u64),
        },
    ]);
    queue.post(
        Role::Client,
        batch(Role::Provider, CONNECTION, frames).encode(),
    );
    queue.step();

    assert_eq!(ready_output(stream.start()), Ok(Phase::Active));
    assert!(poll(&mut pin!(stream.next())).is_pending());
    queue.run_tasks();
    assert_eq!(received.borrow().len(), 1025);
    assert_eq!(ready_output(stream.next()), Some(Ok(1)));
    assert_eq!(ready_output(stream.end()), Ok(1));
}
