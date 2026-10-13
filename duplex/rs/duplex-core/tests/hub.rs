use std::cell::Cell;
use std::rc::Rc;

use duplex_core::{Bundle, Channel, ChannelOptions, ErrorCode, Protocol, SendOptions};
use duplex_test_support::Memory;
use duplex_test_support::conformance::calculator::{
    Add, Calculation, Calculator, CalculatorRouter, Count, User,
};
use duplex_test_support::conformance::login::{
    Credentials, Login, Member, Service, ServiceRouter, Session, Whoami,
};
use duplex_test_support::conformance::{Adder, Directory, credentials};
use duplex_test_support::frames::{AliasTable, Key};
use duplex_test_support::future::ready_output;

use crate::support::hub::{counting, keyed};
use crate::support::log;
use crate::support::validation::progress;

#[test]
fn should_write_what_its_channels_send_in_one_task_as_one_batch() {
    let (near, _far, queue) = Memory::transports();
    let _user = Channel::<User, _>::new(&near, ChannelOptions::default());
    let _member = Channel::<Member, _>::open(&near, ChannelOptions::default(), &credentials())
        .expect("the login channel writes its opening credentials");
    let posted = queue.take();

    assert_eq!(posted.len(), 1);

    let channels: Vec<u32> = AliasTable::default()
        .read(&posted[0])
        .iter()
        .map(|slice| slice.channel)
        .collect();

    assert_eq!(
        channels,
        [
            <Calculation as Protocol>::CHANNEL_ID,
            <Login as Protocol>::CHANNEL_ID
        ]
    );
}

#[test]
fn should_announce_an_alias_once_then_use_it() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();
    let mut aliases = AliasTable::default();
    let keys: Vec<Key> = ["first", "second"]
        .into_iter()
        .flat_map(|text| {
            user.send(&log(text)).expect("the notification is sent");

            queue.take()
        })
        .flat_map(|batch| aliases.read_with_keys(&batch))
        .map(|(key, _)| key)
        .collect();

    assert_eq!(keys, [Key::Announced(1), Key::Aliased(1)]);
}

#[test]
fn should_give_the_alias_of_an_ended_connection_again() {
    let (near, far, queue) = Memory::transports();
    let calculator = Channel::<Calculator, _>::new(&far, ChannelOptions::default());
    let first = Channel::<User, _>::new(&near, ChannelOptions::default());

    queue.deliver();

    let mut aliases = AliasTable::default();
    let mut keys = || -> Vec<Key> {
        calculator
            .send(&progress(Vec::new()))
            .expect("the notification is sent");

        queue
            .take()
            .iter()
            .flat_map(|batch| aliases.read_with_keys(batch))
            .map(|(key, _)| key)
            .collect()
    };
    let announced = keys();

    first.close(ErrorCode::ClosedChannel);
    queue.deliver();
    drop(first);

    let _second = Channel::<User, _>::new(&near, ChannelOptions::default());

    queue.deliver();

    let again = keys();

    assert_eq!(announced, [Key::Announced(1)]);
    assert_eq!(again, [Key::Announced(1)]);
}

#[test]
fn should_carry_the_channels_of_two_protocols_over_one_transport() {
    let (near, far, queue) = Memory::transports();
    let user = Channel::<User, _>::new(&near, ChannelOptions::default());
    let member = Channel::<Member, _>::open(&near, ChannelOptions::default(), &credentials())
        .expect("the login channel writes its opening credentials");
    let calculator = Channel::<Calculator, _>::new(&far, ChannelOptions::default());

    Adder.attach(&calculator);

    let service = Channel::<Service, _>::new(&far, ChannelOptions::default());

    Directory.attach(&service);
    queue.deliver();

    let sum = user.send(&Add { a: 2, b: 3 }).expect("the request is sent");
    let name = member.send(&Whoami {}).expect("the request is sent");

    queue.deliver();
    assert_eq!(ready_output(sum), Ok(5));
    assert_eq!(ready_output(name), Ok("ada".to_owned()));
}

#[test]
#[should_panic(expected = "already has")]
fn should_refuse_a_second_channel_of_the_same_peer_of_a_protocol() {
    let (near, _far, _queue) = Memory::transports();
    let _first = Channel::<User, _>::new(&near, ChannelOptions::default());
    let _second = Channel::<User, _>::new(&near, ChannelOptions::default());
}

#[test]
fn should_give_each_channel_its_share_of_a_batch() {
    let (near, far, queue) = Memory::transports();
    let user = Channel::<User, _>::new(&near, ChannelOptions::default());
    let member = Channel::<Member, _>::open(&near, ChannelOptions::default(), &credentials())
        .expect("the login channel writes its opening credentials");
    let _calculator = Channel::<Calculator, _>::new(&far, ChannelOptions::default());
    let service = Channel::<Service, _>::new(&far, ChannelOptions::default());

    service.on_connect(|open: Credentials, pending| {
        pending.accept(Session {
            greeting: open.user,
        });
    });
    queue.deliver();

    let long = "x".repeat(30_000);
    let _whoami = member.send(&Whoami {}).expect("the request is sent");

    for _ in 0..3 {
        user.send(&log(&long)).expect("the notification is sent");
    }

    let first = queue.take_batches();

    assert_eq!(first.len(), 1, "a full batch goes out at once");

    let mut aliases = AliasTable::default();
    let frames: Vec<(u32, usize)> = aliases
        .read(&first[0])
        .iter()
        .map(|slice| (slice.channel, slice.frames.len()))
        .collect();

    assert_eq!(
        frames,
        [
            (<Login as Protocol>::CHANNEL_ID, 1),
            (<Calculation as Protocol>::CHANNEL_ID, 1)
        ]
    );

    let rest = queue.take();
    let logs: usize = rest
        .iter()
        .flat_map(|batch| aliases.read(batch))
        .map(|slice| slice.frames.len())
        .sum();

    assert_eq!(logs, 2, "what waited goes in the next batch");
}

#[test]
fn should_write_a_batch_for_each_bundle() {
    let (near, _far, queue) = Memory::transports();
    let _user = Channel::<User, _>::new(&near, keyed(7));
    let _member = Channel::<Member, _>::open(&near, ChannelOptions::default(), &credentials())
        .expect("the login channel writes its opening credentials");
    let mut aliases = AliasTable::default();
    let channels: Vec<Vec<u32>> = queue
        .take()
        .iter()
        .map(|batch| {
            aliases
                .read(batch)
                .iter()
                .map(|slice| slice.channel)
                .collect()
        })
        .collect();

    assert_eq!(
        channels,
        [
            [<Calculation as Protocol>::CHANNEL_ID],
            [<Login as Protocol>::CHANNEL_ID]
        ]
    );
}

#[test]
fn should_send_a_message_in_the_bundle_it_is_sent_with_after_what_its_channel_sent_before() {
    let (user, _calculator, queue) = Memory::connected_pair::<User>();

    user.send(&log("first")).expect("the notification is sent");
    user.send_with(
        &log("second"),
        SendOptions {
            bundle: Some(Bundle::Key(1)),
        },
    )
    .expect("the notification is sent");
    user.send(&log("third")).expect("the notification is sent");

    let mut aliases = AliasTable::default();
    let frames: Vec<usize> = queue
        .take()
        .iter()
        .map(|batch| {
            aliases
                .read(batch)
                .iter()
                .map(|slice| slice.frames.len())
                .sum()
        })
        .collect();

    assert_eq!(
        frames,
        [1, 1, 1],
        "each waits for what the channel sent before it"
    );
}

#[test]
fn should_close_a_channel_attached_after_its_transport_disconnected() {
    let (near, _far, queue) = Memory::transports();

    near.downgrade().on_disconnect();

    let user = Channel::<User, _>::new(&near, ChannelOptions::default());

    queue.run_tasks();
    assert_eq!(user.close_reason(), Some(ErrorCode::ConnectionLoss));
}

#[test]
fn should_let_hooks_registered_right_after_a_channel_see_its_first_state() {
    let (near, _far, queue) = Memory::transports();
    let user = Channel::<User, _>::new(&near, ChannelOptions::default());
    let opened = Rc::new(Cell::new(false));

    {
        let opened = Rc::clone(&opened);

        user.on_open(move |_| opened.set(true));
    }

    queue.run_tasks();
    assert!(opened.get());
}

#[test]
fn should_handle_what_comes_in_once_its_batch_was_read_in_the_order_it_came() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let handled = counting(&calculator);

    user.send(&log("first")).expect("the notification is sent");

    let _sum = user.send(&Add { a: 1, b: 2 }).expect("the message is sent");

    user.send(&log("last")).expect("the notification is sent");
    queue.step();
    assert_eq!(*handled.borrow(), ["first", "1+2", "last"]);
}

#[test]
fn should_handle_at_most_its_budget_in_a_cycle() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let handled = counting(&calculator);
    let _sums: Vec<_> = (0..1500)
        .map(|a| user.send(&Add { a, b: 1 }).expect("the message is sent"))
        .collect();

    queue.step();
    queue.run_task();

    let first_cycle = handled.borrow().len();

    queue.run_tasks();
    assert_eq!(first_cycle, 1024);
    assert_eq!(handled.borrow().len(), 1500);
}

#[test]
fn should_stop_reading_while_too_much_waits_to_be_handled() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let _handled = counting(&calculator);
    let _sums: Vec<_> = (0..5000)
        .map(|a| user.send(&Add { a, b: 1 }).expect("the message is sent"))
        .collect();

    queue.step();

    let paused = queue.is_paused(duplex_core::Role::Provider);

    queue.run_tasks();
    assert!(paused);
    assert!(!queue.is_paused(duplex_core::Role::Provider));
}

#[test]
fn should_reject_a_message_timing_out_while_it_waits_to_be_handled_without_handling_it() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let _handled = counting(&calculator);
    let counted = Rc::new(Cell::new(0));

    {
        let counted = Rc::clone(&counted);

        calculator.handle(move |_: Count, _response, _next| {
            counted.set(counted.get() + 1);
        });
    }

    let _sums: Vec<_> = (0..1100)
        .map(|a| user.send(&Add { a, b: 1 }).expect("the message is sent"))
        .collect();
    let mut stream = user.send(&Count { to: 3 }).expect("the message is sent");

    queue.step();
    queue.advance(std::time::Duration::from_secs(2));
    queue.run_tasks();
    queue.deliver();
    assert_eq!(counted.get(), 0);
    assert_eq!(
        ready_output(stream.start()),
        Err(duplex_core::Error::Rejection(ErrorCode::Timeout))
    );
}

#[test]
fn should_send_a_response_given_before_its_channel_closes_before_the_close() {
    let (user, calculator, queue) = Memory::connected_pair::<User>();
    let closing = calculator.clone();

    calculator.handle(move |add: Add, response, _next| {
        response.respond(add.a + add.b);
        closing.close(ErrorCode::ClosedChannel);
    });

    let sum = user.send(&Add { a: 1, b: 2 }).expect("the message is sent");

    queue.deliver();
    assert_eq!(ready_output(sum), Ok(3));
    assert_eq!(user.close_reason(), Some(ErrorCode::ClosedChannel));
}

#[test]
fn should_take_turns_between_its_transports_and_stop_reading_only_the_busy_one() {
    let (busy_user, busy_calculator, busy) = Memory::pair::<User>();
    let (calm_user, calm_calculator, calm) = Memory::pair::<User>();

    busy.deliver();
    calm.deliver();

    let _busy_handled = counting(&busy_calculator);
    let calm_handled = counting(&calm_calculator);
    let _sums: Vec<_> = (0..5000)
        .map(|a| {
            busy_user
                .send(&Add { a, b: 1 })
                .expect("the message is sent")
        })
        .collect();
    let _sum = calm_user
        .send(&Add { a: 1, b: 2 })
        .expect("the message is sent");

    busy.step();
    calm.step();

    let paused = (
        busy.is_paused(duplex_core::Role::Provider),
        calm.is_paused(duplex_core::Role::Provider),
    );
    let calm_before = calm_handled.borrow().len();

    busy.run_task();
    assert_eq!(paused, (true, false));
    assert_eq!(calm_before, 0);
    assert_eq!(*calm_handled.borrow(), ["1+2"]);
    busy.run_tasks();
    assert!(!busy.is_paused(duplex_core::Role::Provider));
}

#[test]
fn should_schedule_every_transport_of_its_thread() {
    let hub = duplex_core::Hub::current();
    let before = hub.transports();
    let (near, far, _queue) = Memory::transports();

    assert_eq!(hub.transports(), before + 2);
    drop((near, far));
    assert_eq!(hub.transports(), before);
}
