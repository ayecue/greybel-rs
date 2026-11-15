//! Host-clock-driven world ticks. Each tick advances world time by
//! [`TIME_SCALE`](greyhack_behaviour::TIME_SCALE) / rate seconds, refills tree budgets, and runs a bounded
//! number of due routines in separate cycles. A late wake processes at most one real second of ticks
//! individually and skips older ticks by advancing the clock. Tick spans with no due routines pass at once.

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

use greyhack_behaviour::{TIME_SCALE, time_after_ticks};

use super::ServerState;
use super::budget::Owner;
use crate::connection;

/// Tick rate and per-tick work limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickOptions {
    /// Tick frequency per real second. The default [`TIME_SCALE`] makes one tick equal one world second.
    pub per_second: u32,
    /// Maximum routines per tick; remaining routines stay due.
    pub routines: usize,
    /// Tokens refilled per tree and tick for contended operations.
    pub operations: u32,
    /// Maximum operation tokens a tree can accumulate.
    pub operation_cap: u32,
}

impl Default for TickOptions {
    fn default() -> Self {
        Self {
            per_second: u32::try_from(TIME_SCALE).expect("value fits the target integer type"),
            routines: 64,
            operations: 32,
            operation_cap: 256,
        }
    }
}

/// Server tick state.
#[derive(Default)]
pub(super) struct TickState {
    pub(super) options: Cell<TickOptions>,
    /// Prevents starting multiple ticker tasks.
    started: Cell<bool>,
    /// Host-time anchor and its associated tick rate.
    anchor: Cell<Option<(Duration, u32)>>,
    /// Number of ticks processed since the anchor.
    count: Cell<u64>,
    /// Ticks since commit, used to persist clock changes at least once per minute.
    since_commit: Cell<u64>,
    /// Remaining ticks to skip after commit failures.
    skip: Cell<u32>,
    /// Exponential backoff applied after the next commit failure.
    backoff: Cell<u32>,
}

/// Maximum tick backoff after a failed commit.
const MAX_BACKOFF: u32 = 64;

impl ServerState {
    /// Starts one ticker task through `spawn` if none is running. Tick counting starts now, and the task
    /// stops when the server is dropped.
    pub(super) fn start_ticks(self: &Rc<Self>, spawn: &connection::Spawn) {
        if self.ticks.started.replace(true) {
            return;
        }

        if self.ticks.anchor.get().is_none() {
            let rate = self.ticks.options.get().per_second.max(1);

            self.ticks.anchor.set(Some((self.clock.now(), rate)));
        }

        let state = Rc::downgrade(self);

        spawn(Box::pin(async move {
            loop {
                let Some(sleep) = state.upgrade().map(|state| {
                    let per_second = state.ticks.options.get().per_second.max(1);

                    state.clock.sleep(Duration::from_secs(1) / per_second)
                }) else {
                    return;
                };

                sleep.await;
                // Immediately completed clock sleeps must still yield to other tasks.
                Yield::default().await;

                let Some(state) = state.upgrade() else {
                    return;
                };

                state.tick().await;
            }
        }));
    }

    /// Processes due ticks, handling at most one real second individually and advancing the clock over
    /// older ticks.
    pub(super) async fn tick(&self) {
        let options = self.ticks.options.get();
        let rate = options.per_second.max(1);
        let mut due = self.due_ticks(rate);
        let limit = u64::from(rate);

        if due == 0 {
            // Run routines already due, including immediate routines scheduled by operations, without a tick.
            self.run_routines(options.routines).await;

            return;
        }

        if due > limit {
            self.pass_ticks(due - limit, rate, false).await;
            due = limit;
        }

        while due > 0 {
            let idle = {
                let state = self.world.lock().await;
                let target = time_after_ticks(state.world.handle.time_seconds(), due, rate);

                match state.world.modules.routines.next_due_at().await {
                    Ok(next) => next.is_none_or(|at| at > target),
                    Err(error) => {
                        self.record_failure(error);

                        false
                    }
                }
            };

            if idle {
                self.pass_ticks(due, rate, false).await;

                return;
            }

            if !self.pass_ticks(1, rate, true).await {
                return;
            }

            self.run_routines(options.routines).await;
            due -= 1;
        }
    }

    /// Calculates unprocessed ticks due at `rate` according to the host clock.
    fn due_ticks(&self, rate: u32) -> u64 {
        let now = self.clock.now();
        let ticks = &self.ticks;
        let (start, anchor_rate) = match ticks.anchor.get() {
            Some(anchor) => anchor,
            None => {
                ticks.anchor.set(Some((now, rate)));

                (now, rate)
            }
        };

        if anchor_rate != rate {
            // Reset the anchor and count when the rate changes.
            ticks.anchor.set(Some((now, rate)));
            ticks.count.set(0);

            return 0;
        }

        let total = now.saturating_sub(start).as_nanos() * u128::from(rate) / 1_000_000_000;

        u64::try_from(total)
            .unwrap_or(u64::MAX)
            .saturating_sub(ticks.count.get())
    }

    /// Advances `count` ticks in one cycle and refills tree budgets. Commits when `commit` is set, after a minute
    /// of ticks, or during backoff. Returns false while commit failures prevent world work.
    async fn pass_ticks(&self, count: u64, rate: u32, commit: bool) -> bool {
        let options = self.ticks.options.get();
        let ticks = &self.ticks;
        let _claim = self.budgets.claim(Owner::World);
        let mut state = self.lock_cycle().await;

        state.world.modules.routines.advance_ticks(count, rate);
        ticks.count.set(ticks.count.get().saturating_add(count));
        self.budgets
            .refill(count, options.operations, options.operation_cap);

        if let Err(error) = state
            .world
            .handle
            .events()
            .publish_now(greyhack_world_core::ClockAdvanceEnd)
            .await
        {
            self.record_failure(error.into());
        }

        if ticks.skip.get() > 0 {
            ticks.skip.set(ticks.skip.get() - 1);

            return false;
        }

        let since_commit = ticks.since_commit.get().saturating_add(count);
        let minute = u64::from(rate) * 60;
        let commits = commit || ticks.backoff.get() > 0 || since_commit >= minute;

        ticks
            .since_commit
            .set(if commits { 0 } else { since_commit });

        if !self.settle(&mut state, commits).await && commits {
            self.refuse_commit();

            return false;
        }

        self.ticks.backoff.set(0);
        drop(state);
        self.send_program_ends();

        true
    }

    /// Runs up to `budget` due routines in separate cycles.
    async fn run_routines(&self, budget: usize) {
        for _ in 0..budget {
            let _claim = self.budgets.claim(Owner::World);
            let mut state = self.lock_cycle().await;
            let mut report = state.world.modules.routines.report();
            let ran = match state.world.modules.routines.run_next_due(&mut report).await {
                Ok(ran) => ran,
                Err(error) => {
                    self.record_failure(error);

                    false
                }
            };

            if !ran {
                return;
            }

            if !self.settle(&mut state, true).await {
                self.refuse_commit();

                return;
            }

            drop(state);
            self.send_program_ends();
        }
    }

    /// Doubles the tick skip interval after a failed commit, up to the backoff limit.
    fn refuse_commit(&self) {
        let backoff = (self.ticks.backoff.get() * 2).clamp(1, MAX_BACKOFF);

        self.ticks.backoff.set(backoff);
        self.ticks.skip.set(backoff);
    }
}

/// Future that yields execution once.
#[derive(Default)]
struct Yield(bool);

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }

        self.0 = true;
        context.waker().wake_by_ref();

        Poll::Pending
    }
}
