//! When an NPC that is late to work wakes up (`NpcInfo.SetLateToWork`).

use crate::npcs::alarm_config::AlarmConfig;
use crate::npcs::npc_schedule::COMMUTE_SECONDS;
use crate::{SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, day_of_year};

/// Seconds after the travel to work starts that an NPC whose alarm is deleted or too late wakes up
/// at the latest (`NpcInfo.SetLateToWork`).
const LATEST_WAKE_UP_SECONDS: u64 = 3 * SECONDS_PER_HOUR;

/// When an NPC that is late to work wakes up and from which day it counts as late.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LateWake {
    /// Seconds from midnight the NPC wakes up at.
    pub wake_up: u64,
    /// Day of year the lateness starts on.
    pub day_alarm: u32,
}

/// Returns when an NPC whose work starts at `work_start` seconds from midnight wakes up at world
/// second `now` with `alarm`, or `None` when the alarm wakes them in time (`NpcInfo.SetLateToWork`).
#[must_use]
pub fn plan_late_wake(work_start: u64, alarm: &AlarmConfig, now: u64) -> Option<LateWake> {
    let travel = work_start - COMMUTE_SECONDS;
    let alarm_time =
        u64::from(alarm.hour) * SECONDS_PER_HOUR + u64::from(alarm.minute) * SECONDS_PER_MINUTE;

    if !alarm.is_deleted && alarm_time <= work_start {
        return None;
    }

    let latest = travel + LATEST_WAKE_UP_SECONDS;
    let wake_up = if alarm.is_deleted || alarm_time > latest {
        latest
    } else {
        alarm_time
    };
    let alarm_day = if now % SECONDS_PER_DAY > travel {
        now + SECONDS_PER_DAY
    } else {
        now
    };

    Some(LateWake {
        wake_up,
        day_alarm: day_of_year(alarm_day),
    })
}
