//! Lateness of NPCs to work and the strikes it brings (`NpcInfo.IsLateToWork`, `NpcInfo.ResetLateToWork`).

use crate::{SECONDS_PER_DAY, day_of_year};

/// Late-to-work completions after which an NPC is fired (`NpcInfo.ResetLateToWork`).
pub const LATE_STRIKES_TO_FIRE: u8 = 3;
/// Days after hiring an NPC counts as hired recently (`NpcInfo.IsHiredRecently`).
pub const HIRED_RECENTLY_DAYS: u64 = 30;

/// Whether an NPC is late to work and whether a lateness it completed counts now
/// (`NpcInfo.IsLateToWork`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lateness {
    /// The NPC has not woken up yet.
    pub late: bool,
    /// The NPC woke up after being late, so the lateness counts as a strike.
    pub completed: bool,
}

/// Returns the lateness of an NPC that wakes up at `wake_up` (seconds from midnight) from day of
/// year `day_alarm`, at world second `at` (`NpcInfo.IsLateToWork`).
#[must_use]
pub fn lateness(wake_up: Option<u64>, day_alarm: u32, at: u64) -> Lateness {
    if day_alarm > day_of_year(at) {
        return Lateness {
            late: false,
            completed: false,
        };
    }

    let late = wake_up.is_some_and(|wake_up| wake_up > at % SECONDS_PER_DAY);

    Lateness {
        late,
        completed: !late && wake_up.is_some(),
    }
}

/// Returns the world second after `now` at which an NPC that wakes up at `wake_up` (seconds from
/// midnight) on day of year `day_alarm` stops being late, or `None` when it has no such wake-up.
/// Lateness counts from the alarm day (`NpcInfo.IsLateToWork`); an alarm day after tomorrow has no
/// wake-up yet.
#[must_use]
pub fn pending_wake_up(wake_up: Option<u64>, day_alarm: u32, now: u64) -> Option<u64> {
    let today = day_of_year(now);
    let midnight = now / SECONDS_PER_DAY * SECONDS_PER_DAY;
    let alarm_day = if day_alarm == today + 1 {
        midnight + SECONDS_PER_DAY
    } else if day_alarm <= today {
        midnight
    } else {
        return None;
    };

    wake_up
        .map(|wake_up| alarm_day + wake_up)
        .filter(|wake_up| *wake_up > now)
}

/// Whether an NPC with `strikes` completed lateness is fired for them (`NpcInfo.ResetLateToWork`).
#[must_use]
pub const fn fires_for_lateness(strikes: u8) -> bool {
    strikes >= LATE_STRIKES_TO_FIRE
}

/// Whether an NPC hired at `hire_time` was hired recently at world second `at`
/// (`NpcInfo.IsHiredRecently`).
#[must_use]
pub fn is_hired_recently(hire_time: Option<u64>, at: u64) -> bool {
    hire_time.is_some_and(|hire_time| {
        at.saturating_sub(hire_time) < HIRED_RECENTLY_DAYS * SECONDS_PER_DAY
    })
}
