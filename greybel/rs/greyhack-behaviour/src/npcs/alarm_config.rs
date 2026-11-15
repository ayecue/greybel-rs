//! NPC smartphone alarm configuration (`AlarmConfig`).

use crate::npcs::npc_schedule::COMMUTE_SECONDS;
use crate::{SECONDS_PER_HOUR, SECONDS_PER_MINUTE};

/// Alarm configured through the phone clock program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AlarmConfig {
    pub hour: u8,
    pub minute: u8,
    pub is_modified: bool,
    pub is_deleted: bool,
}

impl AlarmConfig {
    /// Sets the alarm and schedules lateness (`AlarmConfig.SetAlarmConfig`).
    pub fn set(&mut self, hour: u8, minute: u8, delete: bool) {
        self.hour = hour;
        self.minute = minute;
        self.is_modified = true;
        self.is_deleted = delete;
    }

    /// Clears modification and deletion after completed lateness while preserving the displayed
    /// time (`AlarmConfig.Reset`).
    pub fn reset(&mut self) {
        self.is_modified = false;
        self.is_deleted = false;
    }

    /// Whether `hour` and `minute` are a time of day in 24-hour format
    /// (`FileHelperServer.ModifyClockPhoneServerRpc`).
    #[must_use]
    pub const fn is_valid_time(hour: i64, minute: i64) -> bool {
        0 <= hour && hour <= 23 && 0 <= minute && minute <= 59
    }

    /// Shows the time an NPC whose work starts at `work_start` seconds from midnight goes out
    /// (`FileHelperServer.GetConfigClockPhoneServerRpc`) unless the alarm was modified.
    pub fn show_default_time(&mut self, work_start: u64) {
        if self.is_modified {
            return;
        }

        let travel = work_start - COMMUTE_SECONDS;

        self.hour = u8::try_from(travel / SECONDS_PER_HOUR).expect("the alarm hour fits u8");
        self.minute = u8::try_from((travel % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE)
            .expect("the alarm minute fits u8");
    }
}
