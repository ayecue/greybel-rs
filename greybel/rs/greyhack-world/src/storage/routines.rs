//! Scheduled routine rows with kind-specific data. Routines whose computer, NPC, or coin data was
//! deleted are removed when selected.

use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert};
use crate::{CoinId, ComputerId, PersonId, Result, Routine, ScheduledRoutine, TraceId};

/// Routine-kind discriminant in `routines.kind`, also used as ascending execution priority.
const SERVICE_RESTART: i64 = 0;
const COIN_ROUND: i64 = 1;
const NPC_SWITCH: i64 = 2;
const ACTIVE_TRACE: i64 = 3;
const PASSIVE_TRACE: i64 = 4;

/// Routine row and kind-specific data, or `None` if that data was deleted.
const ROUTINE_COLUMNS: &str = "routines.id, routines.at, routines.kind, routine_service_restarts.computer_id, \
     routine_service_restarts.service, routine_mining_rounds.coin_id, routine_npc_switches.npc_id, routine_active_traces.trace_id, \
     routine_passive_traces.routine_id AS passive_id FROM routines \
     LEFT JOIN routine_service_restarts ON routine_service_restarts.routine_id = routines.id \
     LEFT JOIN routine_mining_rounds ON routine_mining_rounds.routine_id = routines.id \
     LEFT JOIN routine_npc_switches ON routine_npc_switches.routine_id = routines.id \
     LEFT JOIN routine_active_traces ON routine_active_traces.routine_id = routines.id \
     LEFT JOIN routine_passive_traces ON routine_passive_traces.routine_id = routines.id";

/// Routine row converted to a routine, or `None` if its data is missing.
struct RoutineRow {
    id: u64,
    at: u64,
    routine: Option<Routine>,
}

impl FromRow for RoutineRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        let routine = match row.get::<i64>("kind")? {
            SERVICE_RESTART => row
                .get::<Option<u32>>("computer_id")?
                .zip(row.get("service")?)
                .map(|(computer, service)| Routine::ServiceRestart {
                    computer: ComputerId(computer),
                    service,
                }),
            COIN_ROUND => row
                .get::<Option<u32>>("coin_id")?
                .map(|coin| Routine::CoinRound { coin: CoinId(coin) }),
            ACTIVE_TRACE => row
                .get::<Option<u32>>("trace_id")?
                .map(|trace| Routine::ActiveTrace {
                    trace: TraceId(trace),
                }),
            PASSIVE_TRACE => row
                .get::<Option<i64>>("passive_id")?
                .map(|_| Routine::PassiveTrace),
            _ => row
                .get::<Option<u32>>("npc_id")?
                .map(|npc| Routine::NpcSwitch { npc: PersonId(npc) }),
        };

        Ok(Self {
            id: row.get::<i64>("id")?.cast_unsigned(),
            at: row.get("at")?,
            routine,
        })
    }
}

impl Database {
    /// Schedules `routine` at world second `at` and returns its ID.
    pub(crate) async fn insert_routine(&self, at: u64, routine: &Routine) -> Result<u64> {
        let kind = match routine {
            Routine::ServiceRestart { .. } => SERVICE_RESTART,
            Routine::CoinRound { .. } => COIN_ROUND,
            Routine::NpcSwitch { .. } => NPC_SWITCH,
            Routine::ActiveTrace { .. } => ACTIVE_TRACE,
            Routine::PassiveTrace => PASSIVE_TRACE,
        };
        let id = insert(
            &self.conn,
            "INSERT INTO routines (at, kind) VALUES (?1, ?2)",
            params![at.cast_signed(), kind],
        )
        .await?;

        match routine {
            Routine::ServiceRestart { computer, service } => {
                execute(
                    &self.conn,
                    "INSERT INTO routine_service_restarts (routine_id, computer_id, service) VALUES (?1, ?2, ?3)",
                    params![id, computer.0, service.code()],
                )
                .await?;
            }
            Routine::CoinRound { coin } => {
                execute(
                    &self.conn,
                    "INSERT INTO routine_mining_rounds (routine_id, coin_id) VALUES (?1, ?2)",
                    params![id, coin.0],
                )
                .await?;
            }
            Routine::NpcSwitch { npc } => {
                execute(
                    &self.conn,
                    "INSERT INTO routine_npc_switches (routine_id, npc_id) VALUES (?1, ?2)",
                    params![id, npc.0],
                )
                .await?;
            }
            Routine::ActiveTrace { trace } => {
                execute(
                    &self.conn,
                    "INSERT INTO routine_active_traces (routine_id, trace_id) VALUES (?1, ?2)",
                    params![id, trace.0],
                )
                .await?;
            }
            Routine::PassiveTrace => {
                execute(
                    &self.conn,
                    "INSERT INTO routine_passive_traces (routine_id) VALUES (?1)",
                    params![id],
                )
                .await?;
            }
        }

        Ok(id.cast_unsigned())
    }

    pub(crate) async fn remove_routine(&self, id: u64) -> Result<bool> {
        Ok(execute(
            &self.conn,
            "DELETE FROM routines WHERE id = ?1",
            params![id.cast_signed()],
        )
        .await?
            > 0)
    }

    /// Returns the next routine due at `now`, ordered by late-adjusted priority and due time.
    /// Priority rises one level per `raise_after` seconds late. Rows with missing data are deleted
    /// during selection.
    pub(crate) async fn next_due_routine(
        &self,
        now: u64,
        raise_after: u64,
    ) -> Result<Option<ScheduledRoutine>> {
        loop {
            let row: Option<RoutineRow> = fetch_opt(
                &self.conn,
                &format!(
                    "SELECT {ROUTINE_COLUMNS} WHERE routines.at <= ?1 \
                     ORDER BY MAX(routines.kind - (?1 - routines.at) / ?2, 0), routines.at, routines.id LIMIT 1"
                ),
                params![now.cast_signed(), raise_after.max(1).cast_signed()],
            )
            .await?;
            let Some(row) = row else {
                return Ok(None);
            };

            match row.routine {
                Some(routine) => {
                    return Ok(Some(ScheduledRoutine {
                        id: row.id,
                        at: row.at,
                        routine,
                    }));
                }
                None => {
                    self.remove_routine(row.id).await?;
                }
            }
        }
    }

    pub(crate) async fn next_routine_at(&self) -> Result<Option<u64>> {
        Ok(fetch_value(
            &self.conn,
            "SELECT at FROM routines ORDER BY at LIMIT 1",
            (),
        )
        .await?)
    }

    /// Returns all routines with existing data in due-time order.
    pub(crate) async fn routines(&self) -> Result<Vec<ScheduledRoutine>> {
        let rows: Vec<RoutineRow> = fetch_all(
            &self.conn,
            &format!("SELECT {ROUTINE_COLUMNS} ORDER BY routines.at, routines.id"),
            (),
        )
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                row.routine.map(|routine| ScheduledRoutine {
                    id: row.id,
                    at: row.at,
                    routine,
                })
            })
            .collect())
    }

    pub(crate) async fn coin_round(&self, coin: CoinId) -> Result<Option<u64>> {
        Ok(fetch_value::<i64>(
            &self.conn,
            "SELECT routine_id FROM routine_mining_rounds WHERE coin_id = ?1",
            params![coin.0],
        )
        .await?
        .map(i64::cast_unsigned))
    }
}
