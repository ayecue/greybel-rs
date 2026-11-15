//! NPC phone alarms and alarm states.

use greyhack_behaviour::AlarmConfig;
use greyhack_world_core::{ComputerId, PersonId, Result};

use crate::{NpcSwitch, ScheduleService};

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

/// Methods of [`World`] for npcs alarm.
impl ScheduleService {
    pub async fn kill_npcs_at_appliance(&self, appliance: ComputerId) -> Result<Vec<PersonId>> {
        let Some(row) = self.computers.computer(appliance).await? else {
            return Ok(Vec::new());
        };
        let mut dead = Vec::new();

        for npc in self.npcs.npcs_in(row.network).await? {
            if npc
                .schedule
                .walks_to_appliance_soon(self.world.time_seconds())
                && self.jobs.schedule_death(&npc).await?
            {
                dead.push(npc.person);
            }
        }

        Ok(dead)
    }

    pub async fn set_npc_alarm_state(
        &self,
        npc: PersonId,
        alarm: AlarmConfig,
        wake_up_time: Option<u64>,
        day_alarm: u32,
        late_strikes: u8,
        fired: bool,
    ) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.alarm = alarm;
        row.wake_up_time = wake_up_time;
        row.day_alarm = day_alarm;
        row.late_strikes = late_strikes;
        row.fired = fired;

        self.npcs.save_npc_job(&row).await
    }

    pub async fn set_phone_alarm(
        &self,
        public_ip: &str,
        user: &str,
        hour: i64,
        minute: i64,
        delete: bool,
    ) -> Result<Option<Result<PhoneAlarm, AlarmFailure>>> {
        if !AlarmConfig::is_valid_time(hour, minute) {
            return Ok(Some(Err(AlarmFailure::IncorrectTime)));
        }

        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };
        let Some(npc) = self.npcs.npc_in_network(router.network, user).await? else {
            return Ok(Some(Err(AlarmFailure::IncompatibleDevice)));
        };

        if !self.npcs.has_smartphone(router.network, npc.person).await? {
            return Ok(Some(Err(AlarmFailure::IncompatibleDevice)));
        }

        let mut npc = npc;

        npc.alarm.set(
            u8::try_from(hour).expect("value fits the target integer type"),
            u8::try_from(minute).expect("value fits the target integer type"),
            delete,
        );
        npc.set_late_to_work(self.world.time_seconds());
        self.npcs.save_npc_job(&npc).await?;

        let next = NpcSwitch::next_action(&npc, self.world.time_seconds());

        NpcSwitch::reschedule_with(&self.world, &self.scheduler, npc.person, next).await?;

        Ok(Some(Ok(PhoneAlarm {
            config: npc.alarm,
            late_strikes: npc.late_strikes,
            fired: npc.fired,
        })))
    }

    pub async fn phone_alarm(&self, public_ip: &str, user: &str) -> Result<Option<PhoneAlarm>> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };
        let npc = self.npcs.npc_in_network(router.network, user).await?;
        let npc = match npc {
            Some(npc) if self.npcs.has_smartphone(router.network, npc.person).await? => npc,
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

        if npc.is_late_to_work(self.world.time_seconds()).completed {
            self.reset_npc_lateness(&npc).await?;
        }

        let mut npc = self.npcs.npc(npc.person).await?.unwrap_or(npc);

        npc.alarm.show_default_time(npc.schedule.work1_start());

        Ok(Some(PhoneAlarm {
            config: npc.alarm,
            late_strikes: npc.late_strikes,
            fired: npc.fired,
        }))
    }
}
