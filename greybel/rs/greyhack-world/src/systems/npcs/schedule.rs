//! NPC logins and schedules: forced logins, lateness, and work hours.

use greyhack_behaviour::FORCED_LOGIN_SECONDS;

use crate::model::{Npc, PersonId};
use crate::{NpcSchedule, Result, World};

impl World {
    /// Sets `npc` login state from a copied Grey Hack network (`Persona.IsLogged`) without login
    /// side effects. Uses the computer's own session.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn set_npc_login(&mut self, npc: PersonId, logged: bool) -> Result<()> {
        self.db.set_npc_online(npc, logged).await
    }

    /// Forces `npc` login until world second `until` (`NpcInfo.dateForceLogin`), or clears forcing
    /// for `None`, matching copied Grey Hack state without side effects. Schedules the next switch
    /// immediately.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_npc_forced_login(&mut self, npc: PersonId, until: Option<u64>) -> Result<()> {
        let Some(mut row) = self.db.npc(npc).await? else {
            return Ok(());
        };

        row.forced_login_until = until;
        self.db.save_npc_job(&row).await?;

        self.reschedule_npc_switch(npc, self.time_seconds).await
    }

    /// Forces `npc` login for one hour (`Persona.ForceLogin`, `NpcInfo.ForceLogin`) after
    /// social-engineering mail. Logs in immediately and triggers administrator network restoration
    /// (`Router.AdminRestoreNetwork`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub(crate) async fn force_npc_login(&mut self, npc: &Npc) -> Result<()> {
        let mut row = npc.clone();

        row.forced_login_until = Some(self.time_seconds + FORCED_LOGIN_SECONDS);
        self.db.save_npc_job(&row).await?;

        if self.is_network_admin(row.person).await? {
            self.restore_admin_network(row.computer).await?;
        }

        self.reschedule_npc_switch(row.person, self.time_seconds)
            .await
    }

    /// Replaces `npc`'s generated work schedule with copied `schedule`.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_npc_schedule(&mut self, npc: PersonId, schedule: NpcSchedule) -> Result<()> {
        let Some(mut row) = self.db.npc(npc).await? else {
            return Ok(());
        };

        row.schedule = schedule;

        self.db.save_npc_job(&row).await
    }

    /// Records `npc`'s completed lateness and fires on the third strike (`NpcInfo.ResetLateToWork`,
    /// `NpcInfo.SetFired`).
    pub(crate) async fn reset_npc_lateness(&mut self, npc: &Npc) -> Result<()> {
        let mut npc = npc.clone();
        let fire = npc.reset_late_to_work();

        self.db.save_npc_job(&npc).await?;

        if fire {
            self.fire(&npc).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/schedule.rs"]
mod tests;
