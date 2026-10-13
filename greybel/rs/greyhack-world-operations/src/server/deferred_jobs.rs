//! Operation responses deferred until scheduled work finishes. For example, `mining` responds after its
//! mining machine wins its block.

use greyhack_world::{WorkEnd, WorkResult, World};
use tokio::sync::oneshot;

use super::ServerState;
use crate::protocol::ProgramId;

/// Pending operation associated with scheduled work.
pub(crate) struct DeferredJob {
    /// Connection that owns the waiting program.
    pub(crate) connection: u32,
    /// Waiting program ID.
    pub(crate) program: ProgramId,
    /// Sends the work result to the waiting operation.
    end: oneshot::Sender<WorkResult>,
}

impl ServerState {
    /// Defers the response of `connection`'s `program` until scheduled `work` (routine, mine job, or
    /// sniffer) ends. The receiver gets the work result; dropping it cancels the wait and its work.
    pub(crate) fn defer_job(
        &self,
        connection: u32,
        program: ProgramId,
        work: u64,
    ) -> oneshot::Receiver<WorkResult> {
        let (end, signal) = oneshot::channel();

        self.deferred_jobs.borrow_mut().insert(
            work,
            DeferredJob {
                connection,
                program,
                end,
            },
        );

        signal
    }

    /// Checks whether any program in the tree of `connection`'s `program` waits for a mine job in `world`.
    /// A tree can mine only once at a time (`HelperScript`'s one mining process per terminal).
    pub(crate) async fn tree_mines(
        &self,
        connection: u32,
        program: ProgramId,
        world: &World,
    ) -> greyhack_world::Result<bool> {
        let Some(tree) = self.programs.borrow().tree(connection, program) else {
            return Ok(false);
        };
        let mine_jobs = world.mine_jobs().await?;
        let programs = self.programs.borrow();

        Ok(self.deferred_jobs.borrow().iter().any(|(&work, job)| {
            programs.tree(job.connection, job.program) == Some(tree)
                && mine_jobs.iter().any(|mine| mine.id == work)
        }))
    }

    /// Sends each result in `ends` to its deferred operation.
    pub(super) fn resolve_jobs(&self, ends: Vec<WorkEnd>) {
        if ends.is_empty() {
            return;
        }

        let mut jobs = self.deferred_jobs.borrow_mut();

        for end in ends {
            if let Some(job) = jobs.remove(&end.id) {
                let _ = job.end.send(end.result);
            }
        }
    }

    /// Cancels jobs selected by `cancels` and their scheduled work, failing their operations. This matches
    /// mining cancellation when a script ends.
    pub(crate) async fn cancel_jobs(
        &self,
        world: &mut World,
        cancels: impl Fn(&DeferredJob) -> bool,
    ) -> greyhack_world::Result<()> {
        let cancellations: Vec<u64> = {
            let mut jobs = self.deferred_jobs.borrow_mut();
            let cancellations: Vec<u64> = jobs
                .iter()
                .filter(|(_, job)| cancels(job))
                .map(|(&work, _)| work)
                .collect();

            for work in &cancellations {
                jobs.remove(work);
            }

            cancellations
        };

        world.cancel_work(&cancellations).await
    }
}
