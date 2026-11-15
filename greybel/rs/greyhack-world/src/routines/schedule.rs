//! Routine queue execution and reporting.

use std::rc::Rc;

use greyhack_behaviour::TIME_SCALE;
use greyhack_world_core::{ComputerId, Result};
use greyhack_world_routines::{Entry, Scheduler, Window};

use super::{ScheduledRoutine, TickReport};

impl RoutineService {
    pub async fn scheduled_routines(&self) -> Result<Vec<ScheduledRoutine>> {
        self.routines.routines().await
    }

    pub async fn next_due_at(&self) -> Result<Option<u64>> {
        self.routines.next_routine_at().await
    }

    pub async fn run_due(self: &Rc<Self>, budget: usize) -> Result<TickReport> {
        let mut report = self.report();

        for _ in 0..budget {
            if !self.run_next_due(&mut report).await? {
                break;
            }
        }

        Ok(report)
    }

    pub async fn run_next_due(self: &Rc<Self>, report: &mut TickReport) -> Result<bool> {
        let conn = self.world.db().connection();
        let window = Window {
            now: self.world.time_seconds(),
            raise_after: TIME_SCALE,
        };
        let Some(entry) = self.scheduler.run_next_due(&conn, window).await? else {
            return Ok(false);
        };

        self.finish_routine(&entry, report);

        Ok(true)
    }

    pub fn report(&self) -> TickReport {
        TickReport {
            at: self.world.time_seconds(),
            date: self.world.date(),
            routines: Vec::new(),
            npc_events: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Cancels routine `id` and reports whether it existed.
    pub(crate) async fn cancel(&self, id: u64) -> Result<bool> {
        let present = self
            .scheduler
            .cancel(&self.world.db().connection(), id)
            .await?;

        if present {
            self.world.works().cancel(id);
        }

        Ok(present)
    }

    /// Runs computer cleanup through every registered routine kind in registration order.
    pub async fn cancel_routines_on(&self, computers: &[ComputerId]) -> Result<()> {
        let computers: Vec<u32> = computers.iter().map(|computer| computer.0).collect();

        Ok(self
            .scheduler
            .cancel_computers(&self.world.db().connection(), &computers)
            .await?)
    }

    fn finish_routine(&self, entry: &Entry, report: &mut TickReport) {
        self.world.works().finish(entry.id, ());
        report.routines.push(entry.key);

        {
            let mut world = self.world.world_mut();

            report.npc_events.append(
                &mut world
                    .state_mut::<greyhack_world_npcs::NpcRoutineState>()
                    .npc_events,
            );
        }

        report.errors.extend(
            self.world
                .world_mut()
                .state_mut::<greyhack_world_services::ServiceRestartReport>()
                .errors
                .drain(..)
                .map(|error| Box::new(error) as Box<dyn crate::RoutineReport>),
        );
        report.errors.extend(
            self.world
                .world_mut()
                .state_mut::<greyhack_world_economy::MineJobReport>()
                .errors
                .drain(..)
                .map(|error| Box::new(error) as Box<dyn crate::RoutineReport>),
        );
    }
}

/// Executes registered routines and advances their world clock.
pub struct RoutineService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) routines: Rc<crate::storage::RoutineRepository>,
    pub(crate) scheduler: Rc<Scheduler>,
}

impl RoutineService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        routines: Rc<crate::storage::RoutineRepository>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            routines,
            scheduler,
        }
    }
}

impl greyhack_world_modules::WorkOwner for RoutineService {
    fn cancel(&self, id: u64) -> greyhack_world_modules::WorkCancelFuture<'_> {
        Box::pin(async move {
            RoutineService::cancel(self, id)
                .await
                .map_err(greyhack_world_modules::EventError::new)
        })
    }
}
