//! Mining subscribers owned by the economy domain.

use greyhack_world_core::Database;
use greyhack_world_modules::WorkId;
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_value, insert};
use turso::params;

use crate::{CoinId, ComputerId, MineJob, Result};

/// Base work ID for mining subscribers.
const MINE_BASE: WorkId = 1 << 40;

const MINE_COLUMNS: &str = "mining_subscribers.id, mining_subscribers.coin_id, subwallets.user AS subwallet, \
     mining_subscribers.computer_id FROM mining_subscribers \
     JOIN subwallets ON subwallets.id = mining_subscribers.subwallet_id";

struct MineJobRow(MineJob);

impl FromRow for MineJobRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self(MineJob {
            id: MINE_BASE + row.get::<i64>("id")?.cast_unsigned(),
            coin: CoinId(row.get("coin_id")?),
            subwallet: row.get("subwallet")?,
            computer: ComputerId(row.get("computer_id")?),
        }))
    }
}

/// Queries of mining-subscriber rows.
pub(crate) struct MiningRepository {
    db: Database,
}

impl MiningRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn insert_mine_subscriber(
        &self,
        coin: CoinId,
        subwallet: &str,
        computer: ComputerId,
    ) -> Result<Option<WorkId>> {
        let Some(subwallet) = fetch_value::<i64>(
            &self.db.connection(),
            "SELECT id FROM subwallets WHERE coin_id = ?1 AND user = ?2",
            params![coin.0, subwallet],
        )
        .await?
        else {
            return Ok(None);
        };
        let id = insert(
            &self.db.connection(),
            "INSERT INTO mining_subscribers (coin_id, subwallet_id, computer_id) VALUES (?1, ?2, ?3)",
            params![coin.0, subwallet, computer.0],
        )
        .await?;

        Ok(Some(MINE_BASE + id.cast_unsigned()))
    }

    pub(crate) async fn mine_subscribers(&self) -> Result<Vec<MineJob>> {
        let rows: Vec<MineJobRow> = fetch_all(
            &self.db.connection(),
            &format!("SELECT {MINE_COLUMNS} ORDER BY mining_subscribers.id"),
            (),
        )
        .await?;

        Ok(rows.into_iter().map(|row| row.0).collect())
    }

    pub(crate) async fn coin_subscribers(&self, coin: CoinId) -> Result<Vec<MineJob>> {
        let rows: Vec<MineJobRow> = fetch_all(
            &self.db.connection(),
            &format!("SELECT {MINE_COLUMNS} WHERE mining_subscribers.coin_id = ?1 ORDER BY mining_subscribers.id"),
            params![coin.0],
        )
        .await?;

        Ok(rows.into_iter().map(|row| row.0).collect())
    }

    pub(crate) async fn mine_subscribers_on(&self, computer: ComputerId) -> Result<Vec<MineJob>> {
        let rows: Vec<MineJobRow> = fetch_all(
            &self.db.connection(),
            &format!("SELECT {MINE_COLUMNS} WHERE mining_subscribers.computer_id = ?1 ORDER BY mining_subscribers.id"),
            params![computer.0],
        )
        .await?;

        Ok(rows.into_iter().map(|row| row.0).collect())
    }

    pub(crate) async fn remove_mine_subscriber(&self, id: WorkId) -> Result<bool> {
        let Some(row) = id
            .checked_sub(MINE_BASE)
            .filter(|row| *row < (1 << 40))
            .map(u64::cast_signed)
        else {
            return Ok(false);
        };

        Ok(execute(
            &self.db.connection(),
            "DELETE FROM mining_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
            > 0)
    }
}
