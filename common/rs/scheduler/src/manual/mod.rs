use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::{Clock, Sleep};

/// Test clock controlled manually. [`ManualClock::advance`] advances time and runs due timers. Clones share clock
/// state.
#[derive(Clone, Default)]
pub struct ManualClock(Rc<State>);

#[derive(Default)]
struct State {
    now: Cell<Duration>,
    due: RefCell<Vec<Due>>,
    sleep_mode: SleepMode,
}

/// Configures [`Clock::sleep`] behavior.
#[derive(Default, Clone, Copy)]
enum SleepMode {
    /// Waits for the clock to reach the sleep deadline.
    #[default]
    Wait,
    /// Resolves immediately without advancing the clock.
    End,
    /// Advances by the requested delay and resolves immediately.
    Skip,
}

struct Due {
    at: Duration,
    /// Records whether the associated timer was dropped.
    active: Rc<Cell<bool>>,
    task: Box<dyn FnOnce()>,
}

/// One-shot [`ManualClock`] timer handle. Drop cancels the pending callback.
pub struct ManualTimer(Rc<Cell<bool>>);

impl Drop for ManualTimer {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl ManualClock {
    /// Creates a zero-time clock whose sleeps wait for [`ManualClock::advance`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a zero-time clock whose sleeps resolve immediately.
    #[must_use]
    pub fn constant() -> Self {
        Self::with(SleepMode::End)
    }

    /// Creates a zero-time clock that advances by each sleep delay and resolves sleeps immediately.
    #[must_use]
    pub fn advance_on_sleep() -> Self {
        Self::with(SleepMode::Skip)
    }

    fn with(sleep_mode: SleepMode) -> Self {
        Self(Rc::new(State {
            sleep_mode,
            ..State::default()
        }))
    }

    /// Returns a timer that never invokes a callback.
    #[must_use]
    pub fn idle() -> ManualTimer {
        ManualTimer(Rc::new(Cell::new(false)))
    }

    /// Runs `task` at the current time plus `delay` unless the returned timer is dropped first.
    pub fn after(&self, delay: Duration, task: Box<dyn FnOnce()>) -> ManualTimer {
        let active = Rc::new(Cell::new(true));

        self.0.due.borrow_mut().push(Due {
            at: self.0.now.get() + delay,
            active: Rc::clone(&active),
            task,
        });

        ManualTimer(active)
    }

    /// Advances by `by` and runs due callbacks in deadline order, setting the clock to each deadline before its
    /// callback runs.
    pub fn advance(&self, by: Duration) {
        let until = self.0.now.get() + by;

        loop {
            let next = {
                let mut due = self.0.due.borrow_mut();

                due.retain(|due| due.active.get());

                due.iter()
                    .enumerate()
                    .filter(|(_, due)| due.at <= until)
                    .min_by_key(|(_, due)| due.at)
                    .map(|(index, _)| index)
                    .map(|index| due.remove(index))
            };
            let Some(due) = next else {
                break;
            };

            self.0.now.set(self.0.now.get().max(due.at));
            (due.task)();
        }

        self.0.now.set(until);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        self.0.now.get()
    }

    fn sleep(&self, delay: Duration) -> Sleep {
        match self.0.sleep_mode {
            SleepMode::Wait => Sleep::until(|fire| self.after(delay, fire)),
            SleepMode::End => Sleep::ready(),
            SleepMode::Skip => {
                self.advance(delay);

                Sleep::ready()
            }
        }
    }
}
