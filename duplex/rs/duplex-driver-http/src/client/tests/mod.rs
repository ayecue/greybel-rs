#[path = "support/mod.rs"]
mod support;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use duplex_test_support::recorder;
use support::{LIMIT, SWEEP, WAITED, poll_after, poll_within};

use super::Client;

#[test]
fn should_hand_what_the_client_posts_to_its_transport_until_it_disconnects() {
    let client = Client::new(LIMIT);
    let (received, record) = recorder();
    let disconnected = Rc::new(RefCell::new(false));

    client.connect(
        Box::new(move |batch| record(batch.as_slice().to_vec())),
        Box::new({
            let disconnected = Rc::clone(&disconnected);

            move || *disconnected.borrow_mut() = true
        }),
    );

    client.receive([7][..].into());
    client.disconnect();
    client.receive([8][..].into());

    assert_eq!(*received.borrow(), [vec![7]]);
    assert!(*disconnected.borrow());
}

#[test]
fn should_be_gone_only_once_idle_without_requests_or_open_streams() {
    let client = Client::new(LIMIT);
    let streaming = client.begin_stream();
    let while_streaming = (0..3).any(|_| client.is_gone_after(SWEEP, SWEEP));

    drop(streaming);

    let after_its_last_request = client.is_gone_after(SWEEP, SWEEP);
    let idle_for_one = client.is_gone_after(SWEEP, SWEEP);
    let idle_past_it = client.is_gone_after(SWEEP, SWEEP);

    assert!(!while_streaming);
    assert!(!after_its_last_request);
    assert!(!idle_for_one);
    assert!(idle_past_it);
}

#[test]
fn should_be_gone_once_its_client_is_too_far_behind() {
    let client = Client::new(4);

    client.send(&[1, 2, 3]);

    client.send(&[4, 5]);

    assert!(client.is_gone_after(SWEEP, Duration::MAX));
}

#[test]
fn should_leave_out_of_the_long_poll_what_a_post_delivered() {
    let client = Client::new(LIMIT);

    client.send(&[1]);

    let delivered = client.deliver_after(0);
    let polling = poll_after(&client, 0);

    client.acknowledge(0);

    let after_a_stale_poll = poll_after(&client, 0);

    client.note_only(0);

    let after_a_lost_response = poll_after(&client, 0);

    assert_eq!(delivered.as_deref(), Some(&[2, 1, 1][..]));
    assert_eq!(polling, 1);
    assert_eq!(after_a_stale_poll, 1);
    assert_eq!(after_a_lost_response, 0);
}

#[test]
fn should_not_respond_to_a_long_poll_while_a_post_is_handled() {
    let client = Client::new(LIMIT);
    let posting = client.begin_post();

    client.send(&[1]);

    let started = std::time::Instant::now();

    poll_within(&client, 0, WAITED);

    let while_posting = started.elapsed();

    drop(posting);

    let started = std::time::Instant::now();

    poll_within(&client, 0, Duration::from_secs(10));

    let after_posting = started.elapsed();

    assert!(while_posting >= WAITED);
    assert!(after_posting < WAITED);
}
