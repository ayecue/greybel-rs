//! Program subscribers for mining pools, network sniffers, and floods. Rows are deleted when
//! opening a world because they live only as long as their programs. Work IDs offset row IDs by
//! table-specific bases to avoid collisions with routine IDs.

use greyhack_world_core::{ComputerId, Database, Result, RouterId};
use greyhack_world_effects::WorkId;
use greyhack_world_storage::sql::{execute, fetch_value, insert};
use turso::params;

/// Base work ID for sniffer subscribers.
const SNIFFER_BASE: WorkId = WorkId::new(2 << 40);
/// Base work ID for flood subscribers.
const FLOOD_BASE: WorkId = WorkId::new(3 << 40);

/// Converts work `id` to a row ID using table base `base`.
fn row_of(id: WorkId, base: WorkId) -> Option<i64> {
    id.get()
        .checked_sub(base.get())
        .filter(|row| *row < (1 << 40))
        .map(u64::cast_signed)
}

/// Queries of [`Database`] for subscribers.
/// Queries of the tables owned by this area.
pub(crate) struct SubscriberRepository {
    db: Database,
}

impl SubscriberRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn insert_sniffer(
        &self,
        computer: ComputerId,
        router: Option<RouterId>,
    ) -> Result<WorkId> {
        let id = insert(
            &self.db.connection(),
            "INSERT INTO sniffer_subscribers (computer_id, router_id) VALUES (?1, ?2)",
            params![computer.0, router.map(|router| router.0)],
        )
        .await?;

        Ok(WorkId::new(SNIFFER_BASE.get() + id.cast_unsigned()))
    }

    pub(crate) async fn sniffers_on(&self, routers: &[RouterId]) -> Result<Vec<WorkId>> {
        let mut sniffers = Vec::new();

        for router in routers {
            let rows: Vec<i64> = greyhack_world_storage::sql::fetch_column(
                &self.db.connection(),
                "SELECT id FROM sniffer_subscribers WHERE router_id = ?1 ORDER BY id",
                params![router.0],
            )
            .await?;

            sniffers.extend(
                rows.into_iter()
                    .map(|row| WorkId::new(SNIFFER_BASE.get() + row.cast_unsigned())),
            );
        }

        sniffers.sort_unstable();
        sniffers.dedup();

        Ok(sniffers)
    }

    pub(crate) async fn active_sniffers_on(&self, computer: ComputerId) -> Result<Vec<WorkId>> {
        let rows: Vec<i64> = greyhack_world_storage::sql::fetch_column(
            &self.db.connection(),
            "SELECT id FROM sniffer_subscribers WHERE computer_id = ?1 ORDER BY id",
            params![computer.0],
        )
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| WorkId::new(SNIFFER_BASE.get() + row.cast_unsigned()))
            .collect())
    }

    pub(crate) async fn sniffer_computer(&self, id: WorkId) -> Result<Option<ComputerId>> {
        let Some(row) = row_of(id, SNIFFER_BASE) else {
            return Ok(None);
        };

        Ok(fetch_value::<u32>(
            &self.db.connection(),
            "SELECT computer_id FROM sniffer_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
        .map(ComputerId))
    }

    pub(crate) async fn remove_sniffer(&self, id: WorkId) -> Result<bool> {
        let Some(row) = row_of(id, SNIFFER_BASE) else {
            return Ok(false);
        };

        Ok(execute(
            &self.db.connection(),
            "DELETE FROM sniffer_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
            > 0)
    }

    pub(crate) async fn insert_flood(
        &self,
        computer: ComputerId,
        origin: &str,
    ) -> Result<(WorkId, bool)> {
        let counted = fetch_value::<i64>(
            &self.db.connection(),
            "SELECT id FROM flood_subscribers WHERE computer_id = ?1 AND origin = ?2 AND counted = 1",
            params![computer.0, origin],
        )
        .await?
        .is_none();
        let id = insert(
            &self.db.connection(),
            "INSERT INTO flood_subscribers (computer_id, origin, counted) VALUES (?1, ?2, ?3)",
            params![computer.0, origin, counted],
        )
        .await?;

        Ok((WorkId::new(FLOOD_BASE.get() + id.cast_unsigned()), counted))
    }

    pub(crate) async fn flood_count(&self, computer: ComputerId) -> Result<u32> {
        Ok(fetch_value::<u32>(
            &self.db.connection(),
            "SELECT COUNT(*) FROM flood_subscribers WHERE computer_id = ?1 AND counted = 1",
            params![computer.0],
        )
        .await?
        .unwrap_or(0))
    }

    pub(crate) async fn clear_floods(&self, computer: ComputerId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM flood_subscribers WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn remove_flood(&self, id: WorkId) -> Result<bool> {
        let Some(row) = row_of(id, FLOOD_BASE) else {
            return Ok(false);
        };
        let Some((computer, origin)) = greyhack_world_storage::sql::fetch_opt::<(u32, String)>(
            &self.db.connection(),
            "SELECT computer_id, origin FROM flood_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
        else {
            return Ok(false);
        };

        execute(
            &self.db.connection(),
            "DELETE FROM flood_subscribers WHERE id = ?1 \
             OR (computer_id = ?2 AND origin = ?3 AND counted = 1)",
            params![row, computer, origin.as_str()],
        )
        .await?;

        Ok(true)
    }
}
