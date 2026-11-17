//! NPC computer-account login and logout schedules.

use greyhack_behaviour::{
    AlarmConfig, CompanyAlign, JobRole, Lateness, NpcSchedule, WorkPerformance, fires_for_lateness,
    is_hired_recently, lateness, plan_late_wake,
};
use greyhack_world_core::{ComputerId, PersonId};

/// NPC `person` with account `user` on `computer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Npc {
    pub person: PersonId,
    pub computer: ComputerId,
    pub user: String,
    pub name: String,
    pub online: bool,
    /// Whether this NPC has a first-rumor chat (`NpcInfo.hasRumor`).
    pub rumor: bool,
    /// World second after which the NPC is dead (`NpcInfo.deadTime`).
    pub dead_at: Option<u64>,
    pub job_role: JobRole,
    /// NPC vehicle plate (`NpcInfo.licensePlate`); half of NPCs have one.
    pub license_plate: Option<String>,
    /// Whether the NPC is fired (`NpcInfo.isFired`).
    pub fired: bool,
    /// Hiring world second (`NpcInfo.hireTime`); `None` for NPCs generated with the network.
    pub hire_time: Option<u64>,
    /// World second through which social engineering forces login (`NpcInfo.dateForceLogin`).
    pub forced_login_until: Option<u64>,
    /// Work schedule (`NpcInfo.schedule`), drawn on hiring and shift changes.
    pub schedule: NpcSchedule,
    /// Alarm wake time in seconds from midnight (`NpcInfo.wakeUpTime`).
    pub wake_up_time: Option<u64>,
    /// Completed lateness count (`NpcInfo.lateStrikes`).
    pub late_strikes: u8,
    /// Day of year for which the alarm was set (`NpcInfo.dayAlarm`).
    pub day_alarm: u32,
    /// Phone alarm (`NpcInfo.alarmConfig`).
    pub alarm: AlarmConfig,
    /// Job performance (`NpcInfo.workPerformance`).
    pub work_performance: WorkPerformance,
    /// Company stance (`NpcInfo.companyAlign`).
    pub company_align: CompanyAlign,
    /// Mail address of the colleague who writes about the zero-day (`NpcInfo.dayZeroContact`).
    pub day_zero_contact: Option<String>,
    /// Insertion order in `NetworkLan.dayZeroNpcRumors`, if aligned for the rumor.
    pub zero_day_index: Option<u32>,
}

impl Npc {
    /// Returns whether the NPC is dead at world second `at` (`NpcInfo.IsDead`).
    #[must_use]
    pub fn is_dead(&self, at: u64) -> bool {
        self.dead_at.is_some_and(|dead_at| at >= dead_at)
    }

    /// Returns whether death, scheduled death, or firing has opened the NPC position
    /// (`NpcInfo.IsUnavailableForWork`).
    #[must_use]
    pub const fn is_unavailable_for_work(&self) -> bool {
        self.fired || self.dead_at.is_some()
    }

    /// Returns whether the NPC is dead or fired at world second `at` (`NpcInfo.IsDeadOrFired`).
    #[must_use]
    pub fn is_dead_or_fired(&self, at: u64) -> bool {
        self.is_dead(at) || self.fired
    }

    /// Returns whether the NPC is logged in at world second `at` (`Persona.IsLogged`): forced login
    /// or at work.
    #[must_use]
    pub fn is_logged(&self, at: u64) -> bool {
        self.forced_login_until.is_some_and(|until| until > at) || self.schedule.is_working(at)
    }

    /// Schedules the next lateness from the phone alarm (`NpcInfo.SetLateToWork`).
    pub fn set_late_to_work(&mut self, now: u64) {
        match plan_late_wake(self.schedule.work1_start(), &self.alarm, now) {
            None => self.wake_up_time = None,
            Some(late) => {
                self.wake_up_time = Some(late.wake_up);
                self.day_alarm = late.day_alarm;
            }
        }
    }

    /// Returns whether the NPC is late at `at` and whether completed lateness should now increment
    /// the strike count (`NpcInfo.IsLateToWork`).
    #[must_use]
    pub fn is_late_to_work(&self, at: u64) -> Lateness {
        lateness(self.wake_up_time, self.day_alarm, at)
    }

    /// Increments completed lateness and returns whether the strike fires the NPC
    /// (`NpcInfo.ResetLateToWork`).
    pub fn reset_late_to_work(&mut self) -> bool {
        self.alarm.reset();
        self.late_strikes += 1;
        self.wake_up_time = None;

        fires_for_lateness(self.late_strikes) && !self.is_unavailable_for_work()
    }

    /// Returns whether hiring occurred recently before world second `at`
    /// (`NpcInfo.IsHiredRecently`).
    #[must_use]
    pub fn is_hired_recently(&self, at: u64) -> bool {
        is_hired_recently(self.hire_time, at)
    }
}

/// Login or logout applied by a world tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcEvent {
    pub npc: PersonId,
    pub online: bool,
}
