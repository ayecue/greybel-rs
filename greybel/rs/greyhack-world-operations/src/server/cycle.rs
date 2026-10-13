//! Cycles: acquiring the world, budgets, settling events, and committing.

use greyhack_behaviour::MILLIS_PER_SECOND;
use greyhack_world::{WorldError, WorldEvent};
use tokio::sync::MutexGuard;

use super::budget::Owner;
use super::{ServerState, WorldState};
use crate::protocol::{ObjectId, ProgramId};

impl ServerState {
    /// Acquires the world in cycle order, runs `apply`, settles events, sends program termination notices,
    /// and releases the world to the next cycle.
    pub(crate) async fn cycle<T>(&self, apply: impl AsyncFnOnce(&mut WorldState) -> T) -> T {
        let mut state = self.lock_cycle().await;
        let result = apply(&mut state).await;

        self.settle(&mut state, true).await;
        self.send_program_ends();

        result
    }

    /// Runs an operation cycle for `connection`'s `program` like [`Self::cycle`] after drawing tree
    /// budget. Calls `respond` after settlement and before termination notices and the next cycle.
    pub(crate) async fn respond<T>(
        &self,
        connection: u32,
        program: Option<ProgramId>,
        apply: impl AsyncFnOnce(&mut WorldState) -> T,
        respond: impl FnOnce(T),
    ) {
        let _claim = self.draw(connection, program).await;
        let mut state = self.lock_cycle().await;
        let result = apply(&mut state).await;

        self.settle(&mut state, true).await;
        respond(result);
        self.send_program_ends();
    }

    /// Runs [`Self::cycle`] for `connection`'s `program` after drawing from its tree's operation budget.
    pub(crate) async fn cycle_for<T>(
        &self,
        connection: u32,
        program: ProgramId,
        apply: impl AsyncFnOnce(&mut WorldState) -> T,
    ) -> T {
        let _claim = self.draw(connection, Some(program)).await;

        self.cycle(apply).await
    }

    /// Draws a budget token for the tree of `connection`'s `program` and returns its world claim. Ended or
    /// unknown programs draw nothing, and their operations fail later.
    pub(super) async fn draw(
        &self,
        connection: u32,
        program: Option<ProgramId>,
    ) -> Option<super::budget::Claim<'_>> {
        let tree = self.programs.borrow().tree(connection, program?)?;
        let claim = self.budgets.claim(Owner::Tree(tree));

        self.budgets
            .draw(tree, self.ticks.options.get().operation_cap)
            .await;

        Some(claim)
    }

    /// Returns a one-tick pause when the tree of `connection`'s `program` waited since its last throttle.
    pub(crate) fn take_throttle(&self, connection: u32, program: ProgramId) -> Option<u32> {
        let tree = self.programs.borrow().tree(connection, program)?;

        self.budgets.take_throttle(tree).then(|| {
            (MILLIS_PER_SECOND as u32).div_ceil(self.ticks.options.get().per_second.max(1))
        })
    }

    /// Returns the configured per-tick routine limit.
    pub(crate) fn routine_budget(&self) -> usize {
        self.ticks.options.get().routines
    }

    /// Acquires the world in cycle order and begins the transaction settled by [`Self::settle`]. If the
    /// database refuses to begin, the cycle continues without a transaction.
    pub(crate) async fn lock_cycle(&self) -> MutexGuard<'_, WorldState> {
        let state = self.world.lock().await;

        if let Err(error) = state.world.begin().await {
            self.record_failure(error);
        }

        state
    }

    /// Stores a database `error` for the next [`WorldServer::save`].
    pub(crate) fn record_failure(&self, error: WorldError) {
        self.failure.replace(Some(error));
    }

    /// Releases `objects` from `connection` after earlier operations finish.
    pub(crate) async fn release(&self, connection: u32, objects: &[ObjectId]) {
        if let Some(table) = self.world.lock().await.objects.get_mut(&connection) {
            table.release(objects);
        }
    }

    /// Commits the cycle transaction. Failed writes are discarded and reported by the next
    /// [`WorldServer::save`]. Returns whether the commit succeeded.
    pub(super) async fn commit(&self, state: &mut WorldState) -> bool {
        match state.world.commit().await {
            Ok(_) => true,
            Err(error) => {
                self.record_failure(error);

                false
            }
        }
    }

    /// Flushes changes after active cycles. Returns false for memory-only worlds or write failures.
    pub(crate) async fn save(&self) -> bool {
        if self.database.is_none() {
            return false;
        }

        let mut state = self.lock_cycle().await;

        self.settle(&mut state, true).await && self.failure.take().is_none()
    }

    /// Settles cycle events in fixed order: 1. interrupt objects on rebooted computers; 2. terminate
    /// programs with ended processes, including descendants and deferred jobs; 3. pin new objects and commit
    /// unless `commit` is false; 4. resolve ended-work jobs after a successful commit. Jobs cancelled in
    /// phase 2 stay cancelled because program termination takes precedence. Returns whether commit succeeded.
    pub(super) async fn settle(&self, state: &mut WorldState, commit: bool) -> bool {
        let mut ends = Vec::new();
        let mut reboots = Vec::new();

        for event in state.world.take_events() {
            match event {
                WorldEvent::Reboot { computer } => {
                    for objects in state.objects.values_mut() {
                        objects.interrupt(computer);
                    }

                    reboots.push(computer);
                }
                WorldEvent::WorkEnd(end) => ends.push(end),
                // Strikes only tell the player's client (`ActiveGameOverClientRpc`); no program sees them.
                WorldEvent::GameOver { .. } => {}
            }
        }

        self.terminate_ended_programs(&mut state.world, &reboots)
            .await;
        self.pending_ends.borrow_mut().extend(ends);

        if !commit {
            if let Err(error) = state.world.rollback().await {
                self.record_failure(error);
            }

            return false;
        }

        let success = self.commit(state).await;

        if success {
            self.resolve_jobs(std::mem::take(&mut *self.pending_ends.borrow_mut()));
        }

        success
    }
}
