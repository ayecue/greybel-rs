//! NPC weekday work blocks and weekend schedule (`NPCSchedule.NPCSchedule`).

use crate::{
    Rng, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, is_weekend, pending_wake_up,
};

/// Seconds an NPC takes to travel between home and work (`NPCSchedule.NPCSchedule`).
pub const COMMUTE_SECONDS: u64 = 30 * SECONDS_PER_MINUTE;
/// Seconds from the death of an NPC being scheduled to it happening (`NpcInfo.ScheduleDeath`); an NPC that
/// walks to the break-room appliance within this time dies of the appliance accident
/// (`SmartApplianceIntrinsics.override_settings`).
pub const DEATH_DELAY_SECONDS: u64 = 5 * SECONDS_PER_MINUTE;
/// Seconds into the break before which the NPC does not walk to the appliance
/// (`NPCSchedule.SetTimeWalkToAppliance`).
const MIN_APPLIANCE_WALK_SECONDS: u64 = 5 * SECONDS_PER_MINUTE;
/// Seconds a social-engineering mail forces an NPC to stay logged in (`NpcInfo.ForceLogin`).
pub const FORCED_LOGIN_SECONDS: u64 = SECONDS_PER_HOUR;
/// Seconds of each work block (`NPCSchedule.NPCSchedule`).
const WORK_BLOCK_SECONDS: u64 = 4 * SECONDS_PER_HOUR;

/// Half-open interval in seconds from the schedule weekday midnight. Night work may end after
/// midnight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WorkBlock {
    start: u64,
    end: u64,
}

/// Two work periods in an NPC day. Saturday and Sunday use Grey Hack's fixed sleep/leisure schedule
/// with no work periods (`NPCSchedule.NPCSchedule`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcSchedule {
    work: [WorkBlock; 2],
    /// Seconds from midnight when the NPC walks to the break-room appliance between work blocks
    /// (`NPCSchedule.timeWalkAppliance`).
    walk_to_appliance: u64,
}

impl NpcSchedule {
    /// Draws two independent quarter-hour offsets for day or night shifts
    /// (`NPCSchedule.NPCSchedule`). Day work starts at 07:00 plus its offset and night work at
    /// 20:00 plus its offset. Grey Hack travel, break, and lunch durations determine each four-hour
    /// block.
    pub fn new(rng: &mut Rng, day_time: bool) -> Self {
        let start_minutes = u64::from(rng.range(0, 3) * 15) * SECONDS_PER_MINUTE;
        let break_minutes = u64::from(rng.range(0, 3) * 15) * SECONDS_PER_MINUTE;
        let start = if day_time {
            7 * SECONDS_PER_HOUR
        } else {
            20 * SECONDS_PER_HOUR
        } + start_minutes;
        let second_start = start + 5 * SECONDS_PER_HOUR + break_minutes;
        let first_end = start + WORK_BLOCK_SECONDS + COMMUTE_SECONDS;
        // `NPCSchedule.SetTimeWalkToAppliance`: draws a break time at least five minutes into the
        // break, in milliseconds.
        let walk_from = first_end + MIN_APPLIANCE_WALK_SECONDS;
        let walk_span = u32::try_from((second_start - walk_from) * 1_000)
            .expect("a break fits u32 milliseconds");
        let walk_to_appliance = walk_from + u64::from(rng.range(0, walk_span - 1)) / 1_000;

        Self {
            work: [
                WorkBlock {
                    start: start + COMMUTE_SECONDS,
                    end: first_end,
                },
                WorkBlock {
                    start: second_start,
                    end: second_start + WORK_BLOCK_SECONDS,
                },
            ],
            walk_to_appliance: walk_to_appliance % SECONDS_PER_DAY,
        }
    }

    /// Builds a schedule with four-hour work blocks at `work1_start` and `work2_start`, in seconds
    /// from midnight, and appliance walk at `walk_to_appliance` (`NPCSchedule.Schedule`,
    /// `NPCSchedule.timeWalkAppliance`). Accepts generated or persisted values.
    #[must_use]
    pub const fn from_schedule(work1_start: u64, work2_start: u64, walk_to_appliance: u64) -> Self {
        Self {
            work: [
                WorkBlock {
                    start: work1_start,
                    end: work1_start + WORK_BLOCK_SECONDS,
                },
                WorkBlock {
                    start: work2_start,
                    end: work2_start + WORK_BLOCK_SECONDS,
                },
            ],
            walk_to_appliance,
        }
    }

    /// First work-block start in seconds from midnight.
    pub const fn work1_start(&self) -> u64 {
        self.work[0].start
    }

    /// Second work-block start in seconds from midnight.
    pub const fn work2_start(&self) -> u64 {
        self.work[1].start
    }

    /// Seconds from midnight at which the break starts, the end of the first work block
    /// (`NPCSchedule.GetActivityTimeStart(Activity.DescansoBreve)`).
    pub const fn break_start(&self) -> u64 {
        self.work[0].end
    }

    /// Break-room appliance walk time in seconds from midnight.
    pub const fn walk_to_appliance(&self) -> u64 {
        self.walk_to_appliance
    }

    /// Returns whether the NPC works the day shift (`NPCSchedule.dayTime`).
    pub const fn is_day_shift(&self) -> bool {
        self.work[0].start < 20 * SECONDS_PER_HOUR
    }

    /// Returns whether the NPC is commuting at world second `at` (`Activity.ViajeTrabajo`,
    /// `Activity.ViajeCasa`): the weekday half-hours before the first work block and after the
    /// second. The car is on the road during these intervals.
    pub fn is_commuting(&self, at: u64) -> bool {
        matches!(
            self.activity(at),
            Activity::TravelToWork | Activity::TravelHome
        )
    }

    /// Returns the activity at world second `at` (`NPCSchedule.GetCurrentActivity`): the first
    /// activity whose interval contains the time of day. Intervals past midnight wrap to the
    /// schedule day start.
    fn activity(&self, at: u64) -> Activity {
        let time = at % SECONDS_PER_DAY;

        if is_weekend(at) {
            return if (8 * SECONDS_PER_HOUR..23 * SECONDS_PER_HOUR).contains(&time) {
                Activity::Leisure
            } else {
                Activity::Sleep
            };
        }

        let start = self.work[0].start - COMMUTE_SECONDS;
        let home = self.work[1].end;
        let day = [
            (start, start + COMMUTE_SECONDS, Activity::TravelToWork),
            (self.work[0].start, self.work[0].end, Activity::Work),
            (self.work[0].end, self.work[1].start, Activity::Break),
            (self.work[1].start, self.work[1].end, Activity::Work),
            (home, home + COMMUTE_SECONDS, Activity::TravelHome),
            (
                home + COMMUTE_SECONDS,
                home + COMMUTE_SECONDS + 5 * SECONDS_PER_HOUR,
                Activity::Leisure,
            ),
        ];

        day.into_iter()
            .find(|(from, to, _)| covers(*from, *to, time))
            .map_or(Activity::Sleep, |(_, _, activity)| activity)
    }

    /// Returns whether the NPC walks to the break-room appliance within five minutes after world
    /// second `at`, the condition used by `SmartApplianceIntrinsics.override_settings` to kill
    /// them. Compares times of day regardless of weekday.
    pub fn walks_to_appliance_soon(&self, at: u64) -> bool {
        let time_of_day = at % SECONDS_PER_DAY;

        time_of_day < self.walk_to_appliance
            && self.walk_to_appliance - time_of_day <= DEATH_DELAY_SECONDS
    }

    /// Returns whether the activity at world second `at` is `Trabajando`, using the weekday or
    /// weekend schedule (`NPCSchedule.GetCurrentActivity`, `NPCSchedule.GetTypeSchedule`,
    /// `Persona.IsLogged`). World time uses the Unix civil calendar; Grey Hack start date
    /// 03/Jan/2000 was Monday.
    pub fn is_working(&self, at: u64) -> bool {
        if is_weekend(at) {
            return false;
        }

        let within_day = at % SECONDS_PER_DAY;

        self.work
            .iter()
            .any(|block| covers(block.start, block.end, within_day))
    }

    /// Returns whether the NPC is working or on the break between work blocks at world second `at`
    /// (`Activity.Trabajando`, `Activity.DescansoBreve`). This controls smartphone reachability
    /// (`LanPcInfo.IsHidden`).
    pub fn is_at_work(&self, at: u64) -> bool {
        !is_weekend(at) && covers(self.work[0].start, self.work[1].end, at % SECONDS_PER_DAY)
    }

    /// Returns the first work-block boundary strictly after `at`, including blocks beyond midnight
    /// and Friday night work ending Saturday. At that boundary, Saturday uses its weekend schedule.
    pub fn next_boundary(&self, at: u64) -> u64 {
        let day = at / SECONDS_PER_DAY;
        let mut next = u64::MAX;

        for offset in 0..8 {
            let midnight = day.saturating_add(offset).saturating_mul(SECONDS_PER_DAY);

            if !is_weekend(midnight) {
                for block in &self.work {
                    for boundary in [block.start, block.end] {
                        let candidate = midnight.saturating_add(boundary);

                        if candidate > at {
                            next = next.min(candidate);
                        }
                    }
                }
            }
        }

        // Every week contains another weekday and therefore another boundary.
        assert_ne!(next, u64::MAX, "a weekly schedule has a next boundary");

        next
    }
}

impl NpcSchedule {
    /// Returns the world second at which the NPC has to be looked at next after world second `now`:
    /// the earliest of the next work-block boundary, the end of a forced login that has not ended,
    /// and a pending wake-up ([`pending_wake_up`]).
    pub fn next_switch(
        &self,
        forced_login_until: Option<u64>,
        wake_up: Option<u64>,
        day_alarm: u32,
        now: u64,
    ) -> u64 {
        let boundary = self.next_boundary(now);
        let next = forced_login_until
            .filter(|until| *until > now)
            .map_or(boundary, |until| until.min(boundary));

        pending_wake_up(wake_up, day_alarm, now).map_or(next, |wake_up| next.min(wake_up))
    }
}

/// NPC schedule activity (`Activity`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Activity {
    TravelToWork,
    Work,
    Break,
    TravelHome,
    Leisure,
    Sleep,
}

/// Returns whether the half-open `start..end` interval, in seconds from a weekday midnight and
/// possibly past the next midnight, contains `within_day` on the same schedule
/// (`NPCSchedule.GetCurrentActivity`).
const fn covers(start: u64, end: u64, within_day: u64) -> bool {
    if start < SECONDS_PER_DAY && end > SECONDS_PER_DAY {
        within_day >= start || within_day < end - SECONDS_PER_DAY
    } else if start >= SECONDS_PER_DAY {
        within_day >= start - SECONDS_PER_DAY && within_day < end - SECONDS_PER_DAY
    } else {
        within_day >= start && within_day < end
    }
}
