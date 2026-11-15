//! NPC logins and schedules: forced logins, lateness, and work hours.

use greyhack_behaviour::{FORCED_LOGIN_SECONDS, NpcSchedule};
use greyhack_world_core::{ComputerId, PersonId, Result};

use crate::{NetworkRestore, Npc, NpcLoginCapture, NpcSwitch};

/// Methods of [`World`] for npcs schedule.
impl ScheduleService {
    pub async fn set_npc_login(&self, npc: PersonId, logged: bool) -> Result<()> {
        self.npcs.npcs.set_npc_online(npc, logged).await
    }

    pub async fn set_npc_forced_login(&self, npc: PersonId, until: Option<u64>) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.forced_login_until = until;
        self.npcs.save_npc_job(&row).await?;

        NpcSwitch::reschedule_with(&self.world, &self.scheduler, npc, self.world.time_seconds())
            .await
    }

    /// Applies social engineering: records the exposed login, then forces an offline NPC online
    /// (`Persona.ProcesaIngenieraSocial`).
    pub async fn npc_social_login(&self, computer: ComputerId, user: &str) -> Result<()> {
        let npc = self.npcs.npc_on_computer(computer, user).await?;

        if npc
            .as_ref()
            .is_some_and(|npc| npc.is_dead(self.world.time_seconds()))
        {
            return Ok(());
        }

        self.login_capture.record_npc_login(computer, user).await?;

        match npc {
            Some(npc) if !npc.is_logged(self.world.time_seconds()) => {
                self.force_npc_login(&npc).await
            }
            _ => Ok(()),
        }
    }

    /// Forces an NPC online temporarily and restores an administrator's network.
    pub async fn force_npc_login(&self, npc: &Npc) -> Result<()> {
        let mut row = npc.clone();

        row.forced_login_until = Some(self.world.time_seconds() + FORCED_LOGIN_SECONDS);
        self.npcs.save_npc_job(&row).await?;

        if self.npcs.is_network_admin(row.person).await? {
            self.restores.restore_admin_network(row.computer).await?;
        }

        NpcSwitch::reschedule_with(
            &self.world,
            &self.scheduler,
            row.person,
            self.world.time_seconds(),
        )
        .await
    }

    pub async fn set_npc_schedule(&self, npc: PersonId, schedule: NpcSchedule) -> Result<()> {
        let Some(mut row) = self.npcs.npc(npc).await? else {
            return Ok(());
        };

        row.schedule = schedule;

        self.npcs.save_npc_job(&row).await
    }

    pub async fn reset_npc_lateness(&self, npc: &Npc) -> Result<()> {
        let mut npc = npc.clone();
        let fire = npc.reset_late_to_work();

        self.npcs.save_npc_job(&npc).await?;

        if fire {
            self.jobs.fire(&npc).await?;
        }

        Ok(())
    }
}

/// NPC work schedules, logins, lateness, and phone alarms.
pub struct ScheduleService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) npcs: greyhack_world_modules::Dep<crate::NpcService>,
    pub(crate) jobs: greyhack_world_modules::Dep<crate::JobService>,
    pub(crate) restores: greyhack_world_modules::Dep<dyn NetworkRestore>,
    pub(crate) login_capture: greyhack_world_modules::Dep<dyn NpcLoginCapture>,
    pub(crate) scheduler: std::rc::Rc<greyhack_world_effects::Scheduler>,
}

impl ScheduleService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        registry: &std::rc::Rc<greyhack_world_modules::Registry>,
        scheduler: std::rc::Rc<greyhack_world_effects::Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers: registry.get(),
            networks: registry.get(),
            npcs: registry.get(),
            jobs: registry.get(),
            restores: registry.get(),
            login_capture: registry.get(),
            scheduler,
        }
    }
}
