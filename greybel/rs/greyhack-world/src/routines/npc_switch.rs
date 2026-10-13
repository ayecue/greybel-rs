//! NPC login and logout routine ([`Routine::NpcSwitch`]).

use greyhack_behaviour::Lateness;

use super::{Routine, TickReport};
use crate::{Npc, NpcEvent, PersonId, Result, World};

impl World {
    /// Applies the login state required by `Persona.IsLogged` for work or forced login, then
    /// schedules the next work-block boundary or earlier forced-login expiration.
    pub(super) async fn switch_npc(&mut self, id: PersonId, report: &mut TickReport) -> Result<()> {
        let Some(mut npc) = self.db.npc(id).await? else {
            return Ok(());
        };

        if !self.db.has_computer(npc.computer).await? {
            return Ok(());
        }

        // Lateness skips login side effects and administrator network restoration, but login state
        // still follows the schedule (`Persona.IsLogged`). Completed lateness increments before
        // login; the third strike fires the NPC (`Persona.NextCita`).
        let Lateness {
            late,
            completed: should_reset,
        } = npc.is_late_to_work(self.time_seconds);

        if should_reset {
            self.reset_npc_lateness(&npc).await?;
            npc = self.db.npc(id).await?.unwrap_or(npc);
        }

        // Dead or fired NPCs log out and never log in again (`Persona.NextCita`).
        let dead = npc.is_dead_or_fired(self.time_seconds);
        let online = !dead && npc.is_logged(self.time_seconds);
        let sessions = self.db.sessions(npc.computer).await?;

        // Administrators restore the network while at work and not late (`Persona.Login`): on
        // arrival and when waking late during work.
        if online && !late && (!npc.online || should_reset) && self.is_network_admin(id).await? {
            self.restore_admin_network(npc.computer).await?;
        }

        if npc.online != online {
            if online {
                if !sessions.contains(&npc.user) {
                    self.db.insert_session(npc.computer, &npc.user).await?;
                }
            } else {
                while self.db.remove_session(npc.computer, &npc.user).await? {}
            }

            self.db.set_npc_online(id, online).await?;
            report.npc_events.push(NpcEvent { npc: id, online });
        }

        // Grey Hack emits NPC credential packets only for social-engineering mail
        // (`Persona.ProcesaIngenieraSocial`), not scheduled logins. Therefore `switch_npc` records
        // no packet; `npc_social_login` handles that flow.
        if dead {
            return Ok(());
        }

        let next = self.next_npc_action(&npc);

        self.schedule(next, Routine::NpcSwitch { npc: id }).await?;

        Ok(())
    }

    /// Returns the earliest schedule boundary, forced-login expiration, or pending wake-up.
    pub(crate) fn next_npc_action(&self, npc: &Npc) -> u64 {
        npc.schedule.next_switch(
            npc.forced_login_until,
            npc.wake_up_time,
            npc.day_alarm,
            self.time_seconds,
        )
    }
}
