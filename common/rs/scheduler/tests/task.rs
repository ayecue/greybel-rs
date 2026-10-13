//! Tests for futures run through a scheduler's deferred callbacks.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use scheduler::{
    Clock, LocalTask, ManualClock, ManualTimer, Scheduler, Sleep, spawn_through_defer,
};

/// The callbacks a scheduler deferred, in order.
type Callbacks = Rc<RefCell<VecDeque<Box<dyn FnOnce()>>>>;

/// A scheduler whose deferred callbacks run when the test says, its time a manual clock's.
#[derive(Clone, Default)]
struct Deferring {
    callbacks: Callbacks,
    clock: ManualClock,
}

impl Deferring {
    /// Runs the deferred callbacks until none is left; how many ran.
    fn run(&self) -> usize {
        let mut count = 0;

        loop {
            let callback = self.callbacks.borrow_mut().pop_front();
            let Some(callback) = callback else {
                return count;
            };

            callback();
            count += 1;
        }
    }
}

impl Clock for Deferring {
    fn now(&self) -> Duration {
        self.clock.now()
    }

    fn sleep(&self, delay: Duration) -> Sleep {
        Sleep::after(self, delay)
    }
}

impl Scheduler for Deferring {
    type Timer = ManualTimer;

    fn defer(&self, task: Box<dyn FnOnce()>) {
        self.callbacks.borrow_mut().push_back(task);
    }

    fn after(&self, delay: Duration, task: Box<dyn FnOnce()>) -> ManualTimer {
        self.clock.after(delay, task)
    }

    fn interval(&self, _interval: Duration, _task: Box<dyn Fn()>) -> ManualTimer {
        ManualClock::idle()
    }

    fn spawn(&self, task: LocalTask) {
        spawn_through_defer(self, task);
    }
}

#[test]
fn should_poll_a_deferred_task_first_in_a_callback() {
    let scheduler = Deferring::default();
    let ended = Rc::new(Cell::new(false));
    let end = Rc::clone(&ended);

    scheduler.spawn(Box::pin(async move { end.set(true) }));

    assert!(!ended.get());
    assert_eq!(scheduler.run(), 1);
    assert!(ended.get());
}

#[test]
fn should_poll_a_deferred_task_again_once_woken() {
    let scheduler = Deferring::default();
    let steps = Rc::new(Cell::new(0));
    let (sleeper, step) = (scheduler.clone(), Rc::clone(&steps));

    scheduler.spawn(Box::pin(async move {
        step.set(1);
        sleeper.sleep(Duration::from_secs(1)).await;
        step.set(2);
    }));
    scheduler.run();

    assert_eq!(steps.get(), 1);
    assert_eq!(scheduler.run(), 0);

    scheduler.clock.advance(Duration::from_secs(1));

    assert_eq!(scheduler.run(), 1);
    assert_eq!(steps.get(), 2);
}

#[test]
fn should_defer_one_poll_for_wakes_before_it_runs() {
    let scheduler = Deferring::default();
    let polls = Rc::new(Cell::new(0));
    let count = Rc::clone(&polls);

    scheduler.spawn(Box::pin(std::future::poll_fn(move |context| {
        count.set(count.get() + 1);

        if count.get() < 3 {
            context.waker().wake_by_ref();
            context.waker().wake_by_ref();

            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    })));

    assert_eq!(scheduler.run(), 3);
    assert_eq!(polls.get(), 3);
}
