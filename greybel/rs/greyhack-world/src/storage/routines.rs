//! Queries of the generic routine queue and the data tables shared with their owning domains.

use greyhack_world_core::{Database, Result};

use crate::ScheduledRoutine;

/// Queries of the tables owned by this area.
pub(crate) struct RoutineRepository {
    db: Database,
    scheduler: std::rc::Rc<greyhack_world_effects::Scheduler>,
}

impl RoutineRepository {
    pub(crate) fn new(
        db: Database,
        scheduler: std::rc::Rc<greyhack_world_effects::Scheduler>,
    ) -> Self {
        Self { db, scheduler }
    }

    pub(crate) async fn next_routine_at(&self) -> Result<Option<u64>> {
        Ok(self.scheduler.next_due_at(&self.db.connection()).await?)
    }

    pub(crate) async fn routines(&self) -> Result<Vec<ScheduledRoutine>> {
        Ok(self
            .scheduler
            .entries(&self.db.connection())
            .await?
            .into_iter()
            .map(ScheduledRoutine::from_entry)
            .collect())
    }
}
