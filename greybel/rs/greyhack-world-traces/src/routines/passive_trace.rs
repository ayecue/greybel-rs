//! Passive-trace routine owned by the traces domain.

use std::rc::Rc;

use greyhack_world_core::WorldRef;
use greyhack_world_effects::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{execute, fetch_column, fetch_value};
use turso::{Connection, params};

use crate::TraceService;
use crate::storage::schema;

/// The passive trace takes its next step.
pub struct PassiveTrace {
    world: WorldRef,
    traces: greyhack_world_modules::Dep<TraceService>,
    scheduler: Rc<Scheduler>,
}

impl PassiveTrace {
    pub(crate) fn new(
        world: &WorldRef,
        traces: greyhack_world_modules::Dep<TraceService>,
        scheduler: Rc<Scheduler>,
    ) -> Self {
        Self {
            world: world.clone(),
            traces,
            scheduler,
        }
    }

    pub(crate) async fn schedule_with(
        world: &WorldRef,
        scheduler: &Scheduler,
        at: u64,
    ) -> greyhack_world_core::Result<RoutineId> {
        Ok(scheduler
            .schedule::<Self>(&world.db().connection(), at, &())
            .await?)
    }

    /// Schedules the next passive-trace step.
    pub async fn schedule(&self, at: u64) -> greyhack_world_core::Result<RoutineId> {
        Self::schedule_with(&self.world, &self.scheduler, at).await
    }

    pub(crate) async fn stop_with(
        world: &WorldRef,
        scheduler: &Scheduler,
    ) -> greyhack_world_core::Result<()> {
        let conn = world.db().connection();
        let rows: Vec<i64> =
            fetch_column(&conn, "SELECT routine_id FROM routine_passive_traces", ()).await?;

        for row in rows {
            scheduler
                .cancel(&conn, RoutineId::new(row.cast_unsigned()))
                .await?;
        }

        Ok(())
    }

    /// Cancels every queued passive-trace step.
    pub async fn stop(&self) -> greyhack_world_core::Result<()> {
        Self::stop_with(&self.world, &self.scheduler).await
    }
}

impl Routine for PassiveTrace {
    type Data = ();

    const KEY: &'static str = "traces.passive_trace";
    const PRIORITY: i64 = 4;

    fn tables() -> TableSet {
        schema::PASSIVE_TRACES
    }

    async fn store(&self, conn: &Connection, id: RoutineId, (): &()) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_passive_traces (routine_id) VALUES (?1)",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<()>> {
        let row = fetch_value::<i64>(
            conn,
            "SELECT routine_id FROM routine_passive_traces WHERE routine_id = ?1",
            params![id.get().cast_signed()],
        )
        .await?;

        Ok(row.map(|_| ()))
    }

    async fn run(&self, _id: RoutineId, (): ()) -> RoutineResult<()> {
        self.traces
            .run_passive_trace()
            .await
            .map_err(RoutineError::new)
    }
}
