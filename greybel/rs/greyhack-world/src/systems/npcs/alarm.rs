//! NPC phone alarms and alarm states.

use crate::model::{ComputerId, PersonId};
use crate::{AlarmConfig, Result, World};

/// Phone alarm errors (`FileHelperServer.ModifyClockPhoneServerRpc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmFailure {
    /// The time must use 24-hour format.
    IncorrectTime,
    /// The selected device is not an NPC smartphone.
    IncompatibleDevice,
}

/// NPC phone alarm and lateness state exposed through the admin object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhoneAlarm {
    pub config: AlarmConfig,
    pub late_strikes: u8,
    pub fired: bool,
}

impl World {
    /// Schedules death five minutes ahead for NPCs on `appliance`'s network who will walk to the
    /// break-room appliance within five minutes (`SmartApplianceIntrinsics.TriggerAccidentIfReady`,
    /// `NpcInfo.ScheduleDeath`). Processes each NPC once and skips dead or fired NPCs. Death causes
    /// logout.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn kill_npcs_at_appliance(&mut self, appliance: ComputerId) -> Result<Vec<PersonId>> {
        let Some(row) = self.db.computer(appliance).await? else {
            return Ok(Vec::new());
        };
        let mut dead = Vec::new();

        for npc in self.db.npcs_in(row.network).await? {
            if npc.schedule.walks_to_appliance_soon(self.time_seconds)
                && self.schedule_death(&npc).await?
            {
                dead.push(npc.person);
            }
        }

        Ok(dead)
    }

    /// Copies alarm, pending lateness, strikes, and firing state for `npc` from Grey Hack.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_npc_alarm_state(
        &mut self,
        npc: PersonId,
        alarm: AlarmConfig,
        wake_up_time: Option<u64>,
        day_alarm: u32,
        late_strikes: u8,
        fired: bool,
    ) -> Result<()> {
        let Some(mut row) = self.db.npc(npc).await? else {
            return Ok(());
        };

        row.alarm = alarm;
        row.wake_up_time = wake_up_time;
        row.day_alarm = day_alarm;
        row.late_strikes = late_strikes;
        row.fired = fired;

        self.db.save_npc_job(&row).await
    }

    /// Sets the alarm for `user`'s smartphone in network `public_ip`
    /// (`FileHelperServer.ModifyClockPhoneServerRpc`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_phone_alarm(
        &mut self,
        public_ip: &str,
        user: &str,
        hour: i64,
        minute: i64,
        delete: bool,
    ) -> Result<Option<Result<PhoneAlarm, AlarmFailure>>> {
        if !AlarmConfig::is_valid_time(hour, minute) {
            return Ok(Some(Err(AlarmFailure::IncorrectTime)));
        }

        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(npc) = self.npc_in_network(router.network, user).await? else {
            return Ok(Some(Err(AlarmFailure::IncompatibleDevice)));
        };

        if !self.has_smartphone(router.network, npc.person).await? {
            return Ok(Some(Err(AlarmFailure::IncompatibleDevice)));
        }

        let mut npc = npc;

        npc.alarm.set(hour as u8, minute as u8, delete);
        npc.set_late_to_work(self.time_seconds);
        self.db.save_npc_job(&npc).await?;

        let next = self.next_npc_action(&npc);

        self.reschedule_npc_switch(npc.person, next).await?;

        Ok(Some(Ok(PhoneAlarm {
            config: npc.alarm,
            late_strikes: npc.late_strikes,
            fired: npc.fired,
        })))
    }

    /// Reads `user`'s smartphone alarm, records completed lateness, and fires on the third strike
    /// (`FileHelperServer.GetConfigClockPhoneServerRpc`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn phone_alarm(&mut self, public_ip: &str, user: &str) -> Result<Option<PhoneAlarm>> {
        let Some(router) = self.db.router_at(public_ip).await? else {
            return Ok(None);
        };
        let npc = self.npc_in_network(router.network, user).await?;
        let npc = match npc {
            Some(npc) if self.has_smartphone(router.network, npc.person).await? => npc,
            _ => {
                return Ok(Some(PhoneAlarm {
                    config: AlarmConfig {
                        is_deleted: true,
                        ..AlarmConfig::default()
                    },
                    late_strikes: 0,
                    fired: false,
                }));
            }
        };

        if npc.is_late_to_work(self.time_seconds).completed {
            self.reset_npc_lateness(&npc).await?;
        }

        let mut npc = self.db.npc(npc.person).await?.unwrap_or(npc);

        npc.alarm.show_default_time(npc.schedule.work1_start());

        Ok(Some(PhoneAlarm {
            config: npc.alarm,
            late_strikes: npc.late_strikes,
            fired: npc.fired,
        }))
    }
}

#[cfg(test)]
#[path = "tests/alarm.rs"]
mod tests;
