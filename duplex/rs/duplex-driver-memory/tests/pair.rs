//! Tests for in-memory transport pairs using another transport's scheduler or Tokio.

use std::time::Duration;

use duplex_core::{Channel, ChannelOptions, Driver};
use duplex_driver_memory::MemoryDriver;
use duplex_test_support::Memory;
use duplex_test_support::conformance::Adder;
use duplex_test_support::conformance::calculator::{Add, Calculator, CalculatorRouter, User};
use duplex_test_support::future::ready_output;
use duplex_test_support::local::run_local;
use scheduler::{Clock, TokioScheduler};

/// Verifies message delivery over another transport's scheduler and peer disconnection when one endpoint is dropped.
#[test]
fn should_carry_messages_between_the_two_ends_of_a_pair_on_another_transports_scheduler() {
    let (host, _remote, queue) = Memory::transports();
    let (first, second) = MemoryDriver::pair(host.driver().scheduler().clone());
    let user = Channel::<User, _>::new(&first, ChannelOptions::default());
    let calculator = Channel::<Calculator, _>::new(&second, ChannelOptions::default());

    Adder.attach(&calculator);

    let added = user.send(&Add { a: 4, b: 5 }).expect("the request is sent");

    queue.run_tasks();
    assert_eq!(ready_output(added), Ok(9));

    drop(user);
    drop(first);
    queue.run_tasks();
    assert_eq!(calculator.connection(), None);
}

/// Verifies that the pair reads time from the supplied scheduler, using another transport's manually controlled clock.
#[test]
fn should_take_the_time_of_its_scheduler() {
    let (host, _remote, queue) = Memory::transports();
    let (first, _second) = MemoryDriver::pair(host.driver().scheduler().clone());
    let start = first.driver().scheduler().now();

    queue.advance(Duration::from_secs(3));
    assert_eq!(
        first
            .driver()
            .scheduler()
            .now()
            .checked_sub(start)
            .expect("the time goes forward"),
        Duration::from_secs(3)
    );
}

/// Verifies message delivery and disconnection using Tokio tasks on the current `LocalSet`.
#[test]
fn should_carry_messages_between_the_two_ends_of_a_pair_on_tokio() {
    run_local(async {
        let (first, second) = MemoryDriver::pair(TokioScheduler::new());
        let user = Channel::<User, _>::new(&first, ChannelOptions::default());
        let calculator = Channel::<Calculator, _>::new(&second, ChannelOptions::default());

        Adder.attach(&calculator);

        let sum = user
            .send(&Add { a: 20, b: 22 })
            .expect("the request is sent")
            .await;

        assert_eq!(sum, Ok(42));

        drop(user);
        drop(first);
        tokio::time::timeout(Duration::from_secs(5), async {
            while calculator.connection().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the other end loses its connection");
    });
}
