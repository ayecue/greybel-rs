//! NPC login and logout routine owned by the NPC domain.

use std::rc::Rc;

use greyhack_behaviour::Lateness;
use greyhack_world_core::{PersonId, WorldRef};
use greyhack_world_effects::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{execute, fetch_column, fetch_value};
use turso::{Connection, params};

use crate::{Npc, NpcEvent, ScheduleService};

/// NPC login changes emitted while routines run.
#[derive(Default)]
pub struct NpcRoutineState {
    /// Login changes in routine execution order.
    pub npc_events: Vec<NpcEvent>,
}

/// An NPC logs in or out.
pub struct NpcSwitch {
    world: WorldRef,
    schedules: greyhack_world_modules::Dep<ScheduleService>,
    scheduler: Rc<Scheduler>,
}

impl NpcSwitch {
    pub(crate) fn new(
        world: &WorldRef,
        schedules: greyhack_world_modules::Dep<ScheduleService>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            schedules,
            scheduler,
        }
    }

    pub(crate) async fn schedule_with(
        world: &WorldRef,
        scheduler: &Scheduler,
        at: u64,
        npc: PersonId,
    ) -> greyhack_world_core::Result<RoutineId> {
        Ok(scheduler
            .schedule::<Self>(&world.db().connection(), at, &npc)
            .await?)
    }

    /// Schedules an NPC login change.
    pub async fn schedule(&self, at: u64, npc: PersonId) -> greyhack_world_core::Result<RoutineId> {
        Self::schedule_with(&self.world, &self.scheduler, at, npc).await
    }

    pub(crate) async fn reschedule_with(
        world: &WorldRef,
        scheduler: &Scheduler,
        npc: PersonId,
        at: u64,
    ) -> greyhack_world_core::Result<()> {
        let conn = world.db().connection();
        let rows: Vec<i64> = fetch_column(
            &conn,
            "SELECT routine_id FROM routine_npc_switches WHERE npc_id = ?1",
            params![npc.0],
        )
        .await?;

        for row in rows {
            scheduler
                .cancel(&conn, RoutineId::new(row.cast_unsigned()))
                .await?;
        }

        Self::schedule_with(world, scheduler, at, npc).await?;

        Ok(())
    }

    /// Replaces the NPC's queued login change.
    pub async fn reschedule(&self, npc: PersonId, at: u64) -> greyhack_world_core::Result<()> {
        Self::reschedule_with(&self.world, &self.scheduler, npc, at).await
    }

    pub(crate) fn next_action(npc: &Npc, now: u64) -> u64 {
        npc.schedule
            .next_switch(npc.forced_login_until, npc.wake_up_time, npc.day_alarm, now)
    }
}

impl ScheduleService {
    pub(crate) async fn switch_npc(&self, id: PersonId) -> greyhack_world_core::Result<()> {
        let Some(mut npc) = self.npcs.npc(id).await? else {
            return Ok(());
        };

        if !self.computers.has_computer(npc.computer).await? {
            return Ok(());
        }

        // Lateness skips login side effects and administrator network restoration, but login state
        // still follows the schedule (`Persona.IsLogged`). Completed lateness increments before
        // login; the third strike fires the NPC (`Persona.NextCita`).
        let Lateness {
            late,
            completed: should_reset,
        } = npc.is_late_to_work(self.world.time_seconds());

        if should_reset {
            self.reset_npc_lateness(&npc).await?;
            npc = self.npcs.npc(id).await?.unwrap_or(npc);
        }

        // Dead or fired NPCs log out and never log in again (`Persona.NextCita`).
        let dead = npc.is_dead_or_fired(self.world.time_seconds());
        let online = !dead && npc.is_logged(self.world.time_seconds());
        let sessions = self.computers.sessions(npc.computer).await?;

        // Administrators restore the network while at work and not late (`Persona.Login`): on
        // arrival and when waking late during work.
        if online
            && !late
            && (!npc.online || should_reset)
            && self.npcs.is_network_admin(id).await?
        {
            self.restores.restore_admin_network(npc.computer).await?;
        }

        if npc.online != online {
            if online {
                if !sessions.contains(&npc.user) {
                    self.computers
                        .insert_session(npc.computer, &npc.user)
                        .await?;
                }
            } else {
                while self
                    .computers
                    .remove_session(npc.computer, &npc.user)
                    .await?
                {}
            }

            self.npcs.npcs.set_npc_online(id, online).await?;
            self.world
                .world_mut()
                .state_mut::<NpcRoutineState>()
                .npc_events
                .push(NpcEvent { npc: id, online });
        }

        // Scheduled logins do not emit social-engineering credential packets.
        if dead {
            return Ok(());
        }

        let next = NpcSwitch::next_action(&npc, self.world.time_seconds());

        NpcSwitch::schedule_with(&self.world, &self.scheduler, next, id).await?;

        Ok(())
    }
}

impl Routine for NpcSwitch {
    type Data = PersonId;

    const KEY: &'static str = "npcs.switch";
    const PRIORITY: i64 = 2;

    fn tables() -> TableSet {
        crate::storage::schema::routines::NPC_SWITCHES
    }

    async fn store(&self, conn: &Connection, id: RoutineId, data: &PersonId) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_npc_switches (routine_id, npc_id) VALUES (?1, ?2)",
            params![id.get().cast_signed(), data.0],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<PersonId>> {
        let npc = fetch_value::<u32>(
            conn,
            "SELECT npc_id FROM routine_npc_switches WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(npc.map(PersonId))
    }

    async fn run(&self, _id: RoutineId, data: PersonId) -> RoutineResult<()> {
        self.schedules
            .switch_npc(data)
            .await
            .map_err(RoutineError::new)
    }

    async fn cancel_for_computers(
        &self,
        conn: &Connection,
        computers: &[u32],
    ) -> RoutineResult<()> {
        for computer in computers {
            let rows: Vec<i64> = fetch_column(
                conn,
                "SELECT routine_id FROM routine_npc_switches \
                 JOIN npcs ON npcs.person_id = routine_npc_switches.npc_id \
                 JOIN computer_users ON computer_users.id = npcs.user_id \
                 WHERE computer_users.computer_id = ?1",
                params![computer],
            )
            .await?;

            for row in rows {
                let id = RoutineId::new(row.cast_unsigned());

                if self.scheduler.cancel(conn, id).await? {
                    self.world
                        .works()
                        .cancel(greyhack_world_effects::WorkId::from(id));
                }
            }
        }

        Ok(())
    }
}
