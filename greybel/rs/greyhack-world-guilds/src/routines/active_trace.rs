//! Active-trace routine owned by the guilds domain.

use std::rc::Rc;

use greyhack_world_core::WorldRef;
use greyhack_world_routines::{
    Result as RoutineResult, Routine, RoutineError, RoutineId, Scheduler,
};
use greyhack_world_storage::TableSet;
use greyhack_world_storage::sql::{execute, fetch_column, fetch_value};
use turso::{Connection, params};

use crate::storage::schema;
use crate::{TraceId, Traces};

/// An active trace runs out.
pub struct ActiveTrace {
    world: WorldRef,
    traces: greyhack_world_core::Dep<Traces>,
    scheduler: Rc<Scheduler>,
}

impl ActiveTrace {
    pub(crate) fn new(
        world: &WorldRef,
        traces: greyhack_world_core::Dep<Traces>,
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
        trace: TraceId,
    ) -> crate::Result<RoutineId> {
        Ok(scheduler
            .schedule::<Self>(&world.db().connection(), at, &trace)
            .await?)
    }

    /// Schedules the end of an active trace.
    pub async fn schedule(&self, at: u64, trace: TraceId) -> crate::Result<RoutineId> {
        Self::schedule_with(&self.world, &self.scheduler, at, trace).await
    }
}

impl Routine for ActiveTrace {
    type Data = TraceId;

    const KEY: &'static str = "guilds.active_trace";
    const PRIORITY: i64 = 3;

    fn tables() -> TableSet {
        schema::ACTIVE_TRACES
    }

    async fn store(&self, conn: &Connection, id: RoutineId, data: &TraceId) -> RoutineResult<()> {
        execute(
            conn,
            "INSERT INTO routine_active_traces (routine_id, trace_id) VALUES (?1, ?2)",
            params![id.cast_signed(), data.0],
        )
        .await?;

        Ok(())
    }

    async fn load(&self, conn: &Connection, id: RoutineId) -> RoutineResult<Option<TraceId>> {
        let trace = fetch_value::<u32>(
            conn,
            "SELECT trace_id FROM routine_active_traces WHERE routine_id = ?1",
            params![id.cast_signed()],
        )
        .await?;

        Ok(trace.map(TraceId))
    }

    async fn run(&self, _id: RoutineId, data: TraceId) -> RoutineResult<()> {
        self.traces
            .run_active_trace(data)
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
                "SELECT routine_id FROM routine_active_traces \
                 JOIN active_traces ON active_traces.id = routine_active_traces.trace_id \
                 WHERE active_traces.computer_id = ?1",
                params![computer],
            )
            .await?;

            for row in rows {
                let id = row.cast_unsigned();

                if self.scheduler.cancel(conn, id).await? {
                    self.world.works().cancel(id);
                }
            }
        }

        Ok(())
    }
}
