//! World clock: advancing it by seconds, ticks, or budgeted steps, and setting its date.

use super::TickReport;
use crate::Result;
use crate::routines::RoutineService;

/// Methods of [`World`] for routines clock.
impl RoutineService {
    pub fn advance_clock(&self, seconds: u64) {
        let now = self.world.time_seconds();

        self.world.world_mut().time_seconds = now.saturating_add(seconds);
    }

    pub fn advance_ticks(&self, ticks: u64, rate: u32) {
        let (seconds, carry) =
            greyhack_behaviour::advance_ticks(self.world.clock_carry(), ticks, rate);

        self.world.world_mut().clock_carry = carry;
        self.advance_clock(seconds);
    }

    pub async fn tick(self: &std::rc::Rc<Self>, seconds: u64) -> Result<TickReport> {
        self.world
            .events()
            .publish_now(greyhack_world_core::ClockAdvance)
            .await?;
        self.advance_clock(seconds);

        let report = self.run_due(usize::MAX).await?;

        self.world
            .events()
            .publish_now(greyhack_world_core::ClockAdvanceEnd)
            .await?;

        Ok(report)
    }

    pub async fn advance_step(
        self: &std::rc::Rc<Self>,
        target: u64,
        budget: usize,
    ) -> Result<bool> {
        self.world
            .events()
            .publish_now(greyhack_world_core::ClockAdvance)
            .await?;

        let now = self.world.time_seconds();
        let step = self
            .next_due_at()
            .await?
            .filter(|&at| at <= target)
            .map_or(target, |at| at.max(now));
        let mut report = self.report();

        self.advance_clock(step.saturating_sub(now));

        for _ in 0..budget {
            if !self.run_next_due(&mut report).await? {
                break;
            }
        }

        let complete = self.world.time_seconds() >= target
            && self.next_due_at().await?.is_none_or(|at| at > target);

        self.world
            .events()
            .publish_now(greyhack_world_core::ClockAdvanceEnd)
            .await?;

        Ok(complete)
    }

    pub fn set_date(&self, date: &str) -> bool {
        let Some(at) = greyhack_behaviour::try_parse_date(date) else {
            return false;
        };

        self.world.world_mut().time_seconds = at;
        self.world.world_mut().clock_carry = 0;

        true
    }
}

impl crate::World {
    /// Advances world time and routines.
    pub async fn tick(&self, seconds: u64) -> Result<TickReport> {
        self.modules.routines.tick(seconds).await
    }

    /// Advances to the next due routine or target with a bounded routine budget.
    pub async fn advance_step(&self, target: u64, budget: usize) -> Result<bool> {
        self.modules.routines.advance_step(target, budget).await
    }
}
