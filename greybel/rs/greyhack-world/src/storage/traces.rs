//! Active trace, passive trace queue and trace state storage.

use greyhack_behaviour::PassiveStage;
use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert};
use crate::{ActiveTrace, ComputerId, PassiveState, PersonId, Result, TraceId};

impl FromRow for ActiveTrace {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: TraceId(row.get("id")?),
            computer: ComputerId(row.get("computer_id")?),
            admin: PersonId(row.get("admin_id")?),
            terminal: row.get("terminal")?,
            ends_at: row.get("ends_at")?,
        })
    }
}

impl FromRow for PassiveState {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            running: row.get("passive_running")?,
            stage: PassiveStage::from_code(row.get("stage")?),
            from_ip: row.get("from_ip")?,
            to_ip: row.get("to_ip")?,
            origin: row.get::<Option<u32>>("origin_id")?.map(ComputerId),
        })
    }
}

const TRACE_COLUMNS: &str = "id, computer_id, admin_id, terminal, ends_at FROM active_traces";

impl Database {
    /// Inserts an active trace and returns it.
    pub(crate) async fn insert_active_trace(
        &self,
        computer: ComputerId,
        admin: PersonId,
        terminal: u64,
        ends_at: u64,
    ) -> Result<TraceId> {
        let id = insert(
            &self.conn,
            "INSERT INTO active_traces (computer_id, admin_id, terminal, ends_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                computer.0,
                admin.0,
                terminal.cast_signed(),
                ends_at.cast_signed()
            ],
        )
        .await?;

        Ok(TraceId(u32::try_from(id).unwrap_or(u32::MAX)))
    }

    /// Returns the active traces in the order they started.
    pub(crate) async fn active_traces(&self) -> Result<Vec<ActiveTrace>> {
        Ok(fetch_all(
            &self.conn,
            &format!("SELECT {TRACE_COLUMNS} ORDER BY id"),
            (),
        )
        .await?)
    }

    /// Returns active trace `id`.
    pub(crate) async fn active_trace(&self, id: TraceId) -> Result<Option<ActiveTrace>> {
        Ok(fetch_opt(
            &self.conn,
            &format!("SELECT {TRACE_COLUMNS} WHERE id = ?1"),
            params![id.0],
        )
        .await?)
    }

    /// Deletes active trace `id`.
    pub(crate) async fn remove_active_trace(&self, id: TraceId) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM active_traces WHERE id = ?1",
            params![id.0],
        )
        .await?;

        Ok(())
    }

    /// Returns the routine of active trace `id`.
    pub(crate) async fn active_trace_routine(&self, id: TraceId) -> Result<Option<u64>> {
        Ok(fetch_value::<i64>(
            &self.conn,
            "SELECT routine_id FROM routine_active_traces WHERE trace_id = ?1",
            params![id.0],
        )
        .await?
        .map(i64::cast_unsigned))
    }

    /// Returns the computers of the passive trace queue in order.
    pub(crate) async fn passive_traces(&self) -> Result<Vec<ComputerId>> {
        let rows = fetch_all::<super::sql::Scalar<u32>>(
            &self.conn,
            "SELECT computer_id FROM passive_traces ORDER BY id",
            (),
        )
        .await?;

        Ok(rows
            .into_iter()
            .map(|super::sql::Scalar(id)| ComputerId(id))
            .collect())
    }

    /// Appends `computer` to the passive trace queue.
    pub(crate) async fn push_passive_trace(&self, computer: ComputerId) -> Result<()> {
        execute(
            &self.conn,
            "INSERT OR IGNORE INTO passive_traces (computer_id) VALUES (?1)",
            params![computer.0],
        )
        .await?;

        Ok(())
    }

    /// Removes `computer` from the passive trace queue.
    pub(crate) async fn remove_passive_trace(&self, computer: ComputerId) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM passive_traces WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?;

        Ok(())
    }

    /// Empties the passive trace queue.
    pub(crate) async fn clear_passive_traces(&self) -> Result<()> {
        execute(&self.conn, "DELETE FROM passive_traces", ()).await?;

        Ok(())
    }

    /// Returns the passive trace's progress.
    pub(crate) async fn passive_state(&self) -> Result<PassiveState> {
        Ok(fetch_opt::<PassiveState>(
            &self.conn,
            "SELECT passive_running, stage, from_ip, to_ip, origin_id FROM trace_state WHERE id = 0",
            (),
        )
        .await?
        .unwrap_or_default())
    }

    /// Stores the passive trace's progress.
    pub(crate) async fn set_passive_state(&self, state: &PassiveState) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO trace_state (id, passive_running, stage, from_ip, to_ip, origin_id) \
             VALUES (0, ?1, ?2, ?3, ?4, ?5) ON CONFLICT (id) DO UPDATE SET passive_running = ?1, stage = ?2, \
             from_ip = ?3, to_ip = ?4, origin_id = ?5",
            params![
                state.running,
                state.stage.code(),
                state.from_ip.as_str(),
                state.to_ip.as_str(),
                state.origin.map(|computer| computer.0)
            ],
        )
        .await?;

        Ok(())
    }

    /// Returns the player's strikes.
    pub(crate) async fn strikes(&self) -> Result<u8> {
        Ok(fetch_value::<u8>(
            &self.conn,
            "SELECT strikes FROM trace_state WHERE id = 0",
            (),
        )
        .await?
        .unwrap_or(0))
    }

    /// Stores the player's strikes.
    pub(crate) async fn set_strikes(&self, strikes: u8) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO trace_state (id, strikes) VALUES (0, ?1) ON CONFLICT (id) DO UPDATE SET strikes = ?1",
            params![strikes],
        )
        .await?;

        Ok(())
    }

    /// Returns the generation of the player's trace token.
    pub(crate) async fn token_generation(&self) -> Result<u32> {
        Ok(fetch_value::<u32>(
            &self.conn,
            "SELECT token_generation FROM trace_state WHERE id = 0",
            (),
        )
        .await?
        .unwrap_or(0))
    }

    /// Stores the generation of the player's trace token.
    pub(crate) async fn set_token_generation(&self, generation: u32) -> Result<()> {
        execute(
            &self.conn,
            "INSERT INTO trace_state (id, token_generation) VALUES (0, ?1) ON CONFLICT (id) DO UPDATE SET \
             token_generation = ?1",
            params![generation],
        )
        .await?;

        Ok(())
    }
}
