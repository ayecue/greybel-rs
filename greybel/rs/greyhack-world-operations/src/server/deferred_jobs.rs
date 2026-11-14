//! Operation responses deferred until scheduled work finishes.

use greyhack_world::WorkEnvelope;
use tokio::sync::oneshot;

use super::{ServerState, WorldState};
use crate::protocol::ProgramId;

/// Pending operation associated with scheduled work.
pub(crate) struct DeferredJob {
    /// Connection that owns the waiting program.
    pub(crate) connection: u32,
    /// Waiting program ID.
    pub(crate) program: ProgramId,
    /// Sends the type-erased completion to the typed waiting operation.
    end: oneshot::Sender<WorkEnvelope>,
}

impl ServerState {
    /// Defers a cycle-driver wait without selecting its result type at the server boundary.
    pub(crate) fn defer_envelope(
        &self,
        connection: u32,
        program: ProgramId,
        work: u64,
    ) -> oneshot::Receiver<WorkEnvelope> {
        let (end, receiver) = oneshot::channel();

        self.deferred_jobs.borrow_mut().insert(
            work,
            DeferredJob {
                connection,
                program,
                end,
            },
        );

        receiver
    }

    /// Sends each completion to its deferred operation after its owners clear tree state.
    pub(super) fn resolve_jobs(&self, state: &mut WorldState, ends: Vec<WorkEnvelope>) {
        if ends.is_empty() {
            return;
        }

        let owners = state
            .world
            .registry
            .capabilities()
            .all::<dyn greyhack_world_modules::WorkOwner>();

        state.trees.for_each_mut(|_, tree| {
            for end in &ends {
                for owner in &owners {
                    owner.clear_tree(end.id, tree.state_map_mut());
                }
            }
        });

        let mut jobs = self.deferred_jobs.borrow_mut();

        for end in ends {
            if let Some(job) = jobs.remove(&end.id) {
                let _ = job.end.send(end);
            }
        }
    }

    /// Cancels jobs selected by `cancels` and their scheduled work, failing their operations.
    pub(crate) async fn cancel_jobs(
        &self,
        state: &mut WorldState,
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

        let owners = state
            .world
            .registry
            .capabilities()
            .all::<dyn greyhack_world_modules::WorkOwner>();

        state.trees.for_each_mut(|_, tree| {
            for &id in &cancellations {
                for owner in &owners {
                    owner.clear_tree(id, tree.state_map_mut());
                }
            }
        });

        for id in cancellations {
            for owner in &owners {
                if owner.cancel(id).await? {
                    break;
                }
            }
        }

        Ok(())
    }
}
