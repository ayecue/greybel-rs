//! Server operations exposed to wait routes.

use greyhack_world_core::WorldError;
use greyhack_world_effects::{WorkCancel, WorkId};
use greyhack_world_programs::protocol::ProgramId;
use greyhack_world_router::{Call, CycleAccess, RouteContext};

use super::Result;
use crate::World;

impl CyclesExt for CycleAccess<'_> {
    async fn change<T: 'static>(
        self,
        program: ProgramId,
        apply: impl AsyncFnOnce(&RouteContext<'_, World>, Call) -> Result<T>,
    ) -> Result<T> {
        CycleAccess::change_dynamic(self, program, true, apply).await
    }

    async fn world<T: 'static>(self, apply: impl AsyncFnOnce(&World) -> T) -> T {
        CycleAccess::world_dynamic::<T, World, WorldError>(self, apply).await
    }

    async fn defer<T: 'static>(self, program: ProgramId, work: WorkId) -> Result<T, WorkCancel> {
        CycleAccess::defer_dynamic::<T, World, WorldError>(self, program, work.get())
            .await
            .ok_or(WorkCancel)
    }

    fn routine_budget(self) -> usize {
        CycleAccess::routine_budget_dynamic::<World, WorldError>(self)
    }

    async fn save(self) -> bool {
        CycleAccess::save_dynamic::<World, WorldError>(self).await
    }
}

/// World-cycle operations supplied by the server to wait routes.
pub trait CyclesExt {
    /// Runs one budgeted cycle with the waiting program's world and tree.
    async fn change<T: 'static>(
        self,
        program: ProgramId,
        apply: impl AsyncFnOnce(&RouteContext<'_, World>, Call) -> Result<T>,
    ) -> Result<T>;

    /// Runs an unbudgeted world cycle.
    async fn world<T: 'static>(self, apply: impl AsyncFnOnce(&World) -> T) -> T;

    /// Waits for a typed work result.
    async fn defer<T: 'static>(self, program: ProgramId, work: WorkId) -> Result<T, WorkCancel>;

    /// Returns the configured per-tick routine limit.
    fn routine_budget(self) -> usize;

    /// Flushes pending world changes.
    async fn save(self) -> bool;
}
