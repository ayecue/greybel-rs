//! Routine queue scheduling and execution.

use greyhack_behaviour::TIME_SCALE;

use super::{Routine, ScheduledRoutine, TickReport};
use crate::{ComputerId, PersonId, Result, WorkResult, World};

impl World {
    /// Queues `routine` for world second `at` and returns its ID. Routines due now execute on the
    /// next tick, including `tick(0)`.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn schedule(&mut self, at: u64, routine: Routine) -> Result<u64> {
        self.db.insert_routine(at, &routine).await
    }

    /// Returns queued routines in due-time order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn scheduled_routines(&self) -> Result<Vec<ScheduledRoutine>> {
        self.db.routines().await
    }

    /// Returns the earliest queued due time, if any.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn next_due_at(&self) -> Result<Option<u64>> {
        self.db.next_routine_at().await
    }

    /// Runs at most `budget` due routines to completion in priority, due-time, and ID order:
    /// service restarts, coin rounds, then NPC switches. Priority increases by one for every
    /// [`TIME_SCALE`] world seconds late, preventing starvation. Rows are removed only after
    /// execution, so routines stay due after database failures. Rescheduled work is due later, ensuring
    /// termination.
    ///
    /// # Errors
    ///
    /// Returns any database read error. The failed routine remains due; earlier routines remain
    /// completed.
    pub async fn run_due(&mut self, budget: usize) -> Result<TickReport> {
        let mut report = self.report();

        for _ in 0..budget {
            if !self.run_next_due(&mut report).await? {
                break;
            }
        }

        Ok(report)
    }

    /// Runs the next routine selected by [`World::run_due`], appends its results to `report`, and
    /// returns whether a routine was due.
    ///
    /// # Errors
    ///
    /// Returns any database read error. The routine remains due.
    pub async fn run_next_due(&mut self, report: &mut TickReport) -> Result<bool> {
        let Some(entry) = self
            .db
            .next_due_routine(self.time_seconds, TIME_SCALE)
            .await?
        else {
            return Ok(false);
        };

        // Remove the routine row before execution so self-rescheduling inserts a new row instead of
        // colliding with the pending row.
        self.db.remove_routine(entry.id).await?;

        match &entry.routine {
            Routine::ServiceRestart { computer, service } => {
                self.restart_service(*computer, *service, report).await?;
            }
            Routine::NpcSwitch { npc } => self.switch_npc(*npc, report).await?,
            Routine::CoinRound { coin } => self.run_coin_round(*coin, report).await?,
            Routine::ActiveTrace { trace } => self.run_active_trace(*trace).await?,
            Routine::PassiveTrace => self.run_passive_trace().await?,
        }

        self.work.push_end(entry.id, WorkResult::Complete);
        report.routines.push(entry.routine);

        Ok(true)
    }

    /// Creates an empty report at the current world clock for [`World::run_next_due`].
    #[must_use]
    pub fn report(&self) -> TickReport {
        TickReport {
            at: self.time_seconds,
            date: self.date(),
            routines: Vec::new(),
            npc_events: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Moves `npc`'s single next-switch routine to world second `at`.
    pub(crate) async fn reschedule_npc_switch(&mut self, npc: PersonId, at: u64) -> Result<()> {
        for entry in self.scheduled_routines().await? {
            if entry.routine == (Routine::NpcSwitch { npc }) {
                self.db.remove_routine(entry.id).await?;
            }
        }

        self.schedule(at, Routine::NpcSwitch { npc }).await?;

        Ok(())
    }

    /// Cancels service restarts and NPC switches belonging to `computers`, recording each
    /// completion. Coin mining rounds have no computer owner and remain scheduled.
    pub(crate) async fn cancel_routines_on(&mut self, computers: &[ComputerId]) -> Result<()> {
        for entry in self.scheduled_routines().await? {
            let cancel = match &entry.routine {
                Routine::ServiceRestart { computer, .. } => computers.contains(computer),
                Routine::NpcSwitch { npc } => match self.db.npc(*npc).await? {
                    Some(npc) => computers.contains(&npc.computer),
                    None => true,
                },
                Routine::CoinRound { .. } | Routine::PassiveTrace => false,
                Routine::ActiveTrace { trace } => match self.db.active_trace(*trace).await? {
                    Some(trace) => computers.contains(&trace.computer),
                    None => true,
                },
            };

            if cancel {
                self.end_work(entry.id, WorkResult::Cancel).await?;
            }
        }

        Ok(())
    }
}
