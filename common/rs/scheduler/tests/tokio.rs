//! Tests for Tokio clock, timer, and deferred-task behavior.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use scheduler::{Clock, Scheduler, TokioScheduler};
use tokio::task::LocalSet;

fn run_local<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, future)
}

#[test]
fn should_run_deferred_tasks_in_order() {
    run_local(async {
        let scheduler = TokioScheduler::new();
        let order = Rc::new(RefCell::new(Vec::new()));

        for index in 0..3 {
            let order = Rc::clone(&order);

            scheduler.defer(Box::new(move || order.borrow_mut().push(index)));
        }

        tokio::task::yield_now().await;

        assert_eq!(*order.borrow(), [0, 1, 2]);
    });
}

#[test]
fn should_sleep_and_fire_timers_on_tokios_clock() {
    run_local(async {
        let scheduler = TokioScheduler::new();
        let fired = Rc::new(RefCell::new(0));
        let _timer = {
            let fired = Rc::clone(&fired);

            scheduler.interval(
                Duration::from_secs(1),
                Box::new(move || *fired.borrow_mut() += 1),
            )
        };

        scheduler.sleep(Duration::from_millis(3_500)).await;

        assert_eq!(*fired.borrow(), 3);
        assert!(scheduler.now() >= Duration::from_millis(3_500));
    });
}

#[test]
fn should_not_run_a_dropped_timer() {
    run_local(async {
        let scheduler = TokioScheduler::new();
        let fired = Rc::new(RefCell::new(false));
        let timer = {
            let fired = Rc::clone(&fired);

            scheduler.after(
                Duration::from_secs(1),
                Box::new(move || *fired.borrow_mut() = true),
            )
        };

        drop(timer);
        scheduler.sleep(Duration::from_secs(2)).await;

        assert!(!*fired.borrow());
    });
}
