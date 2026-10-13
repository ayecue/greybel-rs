//! Tests for the manual clock.

use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Waker};
use std::time::Duration;

use scheduler::{Clock, ManualClock};

fn is_ready(future: &mut (impl Future<Output = ()> + Unpin)) -> bool {
    std::pin::Pin::new(future)
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_ready()
}

#[test]
fn should_run_the_timers_due_in_the_order_of_their_times_when_advanced() {
    let clock = ManualClock::new();
    let order = Rc::new(Cell::new(0_u32));
    let late = {
        let order = Rc::clone(&order);

        clock.after(
            Duration::from_secs(2),
            Box::new(move || order.set(order.get() * 10 + 2)),
        )
    };
    let early = {
        let order = Rc::clone(&order);

        clock.after(
            Duration::from_secs(1),
            Box::new(move || order.set(order.get() * 10 + 1)),
        )
    };

    clock.advance(Duration::from_secs(3));

    assert_eq!(order.get(), 12);
    assert_eq!(clock.now(), Duration::from_secs(3));
    drop((late, early));
}

#[test]
fn should_wait_for_the_clock_to_pass_the_end_of_a_sleep() {
    let clock = ManualClock::new();
    let mut sleep = clock.sleep(Duration::from_secs(5));

    assert!(!is_ready(&mut sleep));
    clock.advance(Duration::from_secs(4));
    assert!(!is_ready(&mut sleep));
    clock.advance(Duration::from_secs(1));
    assert!(is_ready(&mut sleep));
}

#[test]
fn should_end_sleeps_at_once_on_a_constant_clock_or_one_sleeps_advance() {
    let constant = ManualClock::constant();
    let advancing = ManualClock::advance_on_sleep();

    assert!(is_ready(&mut constant.sleep(Duration::from_secs(5))));
    assert_eq!(constant.now(), Duration::ZERO);
    assert!(is_ready(&mut advancing.sleep(Duration::from_secs(5))));
    assert_eq!(advancing.now(), Duration::from_secs(5));
}

#[test]
fn should_not_run_a_dropped_timer() {
    let clock = ManualClock::new();
    let ran = Rc::new(Cell::new(false));
    let timer = {
        let ran = Rc::clone(&ran);

        clock.after(Duration::from_secs(1), Box::new(move || ran.set(true)))
    };

    drop(timer);
    clock.advance(Duration::from_secs(2));

    assert!(!ran.get());
}
