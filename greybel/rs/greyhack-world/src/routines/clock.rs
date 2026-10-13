//! World clock: advancing it by seconds, ticks, or budgeted steps, and setting its date.

use super::TickReport;
use crate::{Result, World};

impl World {
    /// Advances the world clock by `seconds` without running routines.
    pub fn advance_clock(&mut self, seconds: u64) {
        self.time_seconds = self.time_seconds.saturating_add(seconds);
    }

    /// Advances the clock by `ticks` at `rate` ticks per real second without running routines. Each
    /// tick adds [`greyhack_behaviour::TIME_SCALE`] / `rate` world seconds and carries fractional seconds forward, so
    /// `rate` ticks advance exactly [`greyhack_behaviour::TIME_SCALE`] seconds at every rate.
    pub fn advance_ticks(&mut self, ticks: u64, rate: u32) {
        let (seconds, carry) = greyhack_behaviour::advance_ticks(self.clock_carry, ticks, rate);

        self.clock_carry = carry;
        self.advance_clock(seconds);
    }

    /// Advances by `seconds`, then runs all routines due by the new time, matching `admin.tick`.
    ///
    /// # Errors
    ///
    /// Returns any database read error. The failed routine remains due; earlier routines remain
    /// completed.
    pub async fn tick(&mut self, seconds: u64) -> Result<TickReport> {
        self.update_crown_leaderboard().await?;
        self.advance_clock(seconds);

        let report = self.run_due(usize::MAX).await?;

        self.update_crown_leaderboard().await?;

        Ok(report)
    }

    /// Advances the clock to the next routine due by `target`, or to `target`, then runs up to
    /// `budget` routines. Returns true after reaching `target` with no routine left due by `target`.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn advance_step(&mut self, target: u64, budget: usize) -> Result<bool> {
        let now = self.time_seconds();
        let step = self
            .next_due_at()
            .await?
            .filter(|&at| at <= target)
            .map_or(target, |at| at.max(now));
        let mut report = self.report();

        // Grey Hack evaluates the Crown ranking every minute, so a guild that leads when the clock
        // starts moving is its candidate from then on.
        self.update_crown_leaderboard().await?;
        self.advance_clock(step.saturating_sub(now));

        for _ in 0..budget {
            if !self.run_next_due(&mut report).await? {
                break;
            }
        }

        self.update_crown_leaderboard().await?;

        Ok(self.time_seconds() >= target && self.next_due_at().await?.is_none_or(|at| at > target))
    }

    /// Sets the world clock from `date` in `dd/MMM/yyyy - HH:mm` format. Returns false and retains
    /// the clock unchanged for other formats.
    pub fn set_date(&mut self, date: &str) -> bool {
        let Some(at) = greyhack_behaviour::try_parse_date(date) else {
            return false;
        };

        self.time_seconds = at;
        self.clock_carry = 0;

        true
    }
}
