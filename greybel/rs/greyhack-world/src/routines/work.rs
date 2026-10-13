//! Completion handling for routines, mine jobs, sniffers, and floods.

use crate::world::WorkId;
use crate::{MineJobFailure, Result, WorkResult, World};

impl World {
    /// Ends work `id` with `result`. Supports routines, mine jobs, sniffers, and floods; previously
    /// ended work retains its original result.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub(crate) async fn end_work(&mut self, id: WorkId, result: WorkResult) -> Result<()> {
        let removal = self.db.remove_mine_subscriber(id).await?
            || self.db.remove_sniffer(id).await?
            || self.db.remove_flood(id).await?
            || self.db.remove_routine(id).await?;

        if removal {
            self.work.push_end(id, result);
        }

        Ok(())
    }

    /// Cancels routine, mine-job, or sniffer work `ids`, matching script mining cleanup on exit.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn cancel_work(&mut self, ids: &[WorkId]) -> Result<()> {
        for &id in ids {
            self.end_work(id, WorkResult::Cancel).await?;
        }

        Ok(())
    }

    /// Fails mine jobs `ids` with `failure`.
    pub(crate) async fn fail_work(
        &mut self,
        ids: &[WorkId],
        failure: &MineJobFailure,
    ) -> Result<()> {
        for &id in ids {
            self.end_work(id, WorkResult::Failure(failure.clone()))
                .await?;
        }

        Ok(())
    }
}
