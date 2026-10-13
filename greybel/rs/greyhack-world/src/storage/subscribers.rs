//! Program subscribers for mining pools, network sniffers, and floods. Rows are deleted when
//! opening a world because they live only as long as their programs. Work IDs offset row IDs by
//! table-specific bases to avoid collisions with routine IDs.

use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_value, insert};
use crate::world::WorkId;
use crate::{CoinId, ComputerId, MineJob, Result, RouterId};

/// Base work ID for mining subscribers.
const MINE_BASE: WorkId = 1 << 40;
/// Base work ID for sniffer subscribers.
const SNIFFER_BASE: WorkId = 2 << 40;
/// Base work ID for flood subscribers.
const FLOOD_BASE: WorkId = 3 << 40;

const MINE_COLUMNS: &str = "mining_subscribers.id, mining_subscribers.coin_id, subwallets.user AS subwallet, \
     mining_subscribers.computer_id FROM mining_subscribers \
     JOIN subwallets ON subwallets.id = mining_subscribers.subwallet_id";

impl FromRow for MineJob {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: MINE_BASE + row.get::<i64>("id")?.cast_unsigned(),
            coin: CoinId(row.get("coin_id")?),
            subwallet: row.get("subwallet")?,
            computer: ComputerId(row.get("computer_id")?),
        })
    }
}

/// Converts work `id` to a row ID using table base `base`.
fn row_of(id: WorkId, base: WorkId) -> Option<i64> {
    id.checked_sub(base)
        .filter(|row| *row < (1 << 40))
        .map(u64::cast_signed)
}

impl Database {
    /// Subscribes `computer` to `coin` through `subwallet`. Returns the work ID, or `None` if the
    /// sub-wallet is absent.
    pub(crate) async fn insert_mine_subscriber(
        &self,
        coin: CoinId,
        subwallet: &str,
        computer: ComputerId,
    ) -> Result<Option<WorkId>> {
        let Some(subwallet) = fetch_value::<i64>(
            &self.conn,
            "SELECT id FROM subwallets WHERE coin_id = ?1 AND user = ?2",
            params![coin.0, subwallet],
        )
        .await?
        else {
            return Ok(None);
        };
        let id = insert(
            &self.conn,
            "INSERT INTO mining_subscribers (coin_id, subwallet_id, computer_id) VALUES (?1, ?2, ?3)",
            params![coin.0, subwallet, computer.0],
        )
        .await?;

        Ok(Some(MINE_BASE + id.cast_unsigned()))
    }

    /// Returns all mining subscribers in subscription order.
    pub(crate) async fn mine_subscribers(&self) -> Result<Vec<MineJob>> {
        Ok(fetch_all(
            &self.conn,
            &format!("SELECT {MINE_COLUMNS} ORDER BY mining_subscribers.id"),
            (),
        )
        .await?)
    }

    /// Returns `coin` pool subscribers in join order.
    pub(crate) async fn coin_subscribers(&self, coin: CoinId) -> Result<Vec<MineJob>> {
        Ok(fetch_all(
            &self.conn,
            &format!("SELECT {MINE_COLUMNS} WHERE mining_subscribers.coin_id = ?1 ORDER BY mining_subscribers.id"),
            params![coin.0],
        )
        .await?)
    }

    /// Returns mining subscribers on `computer`.
    pub(crate) async fn mine_subscribers_on(&self, computer: ComputerId) -> Result<Vec<MineJob>> {
        Ok(fetch_all(
            &self.conn,
            &format!(
                "SELECT {MINE_COLUMNS} WHERE mining_subscribers.computer_id = ?1 ORDER BY mining_subscribers.id"
            ),
            params![computer.0],
        )
        .await?)
    }

    /// Ends mining subscriber `id` and returns whether it existed.
    pub(crate) async fn remove_mine_subscriber(&self, id: WorkId) -> Result<bool> {
        let Some(row) = row_of(id, MINE_BASE) else {
            return Ok(false);
        };

        Ok(execute(
            &self.conn,
            "DELETE FROM mining_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
            > 0)
    }

    /// Starts a sniffer on `computer` for `router`'s network and returns its work ID.
    pub(crate) async fn insert_sniffer(
        &self,
        computer: ComputerId,
        router: Option<RouterId>,
    ) -> Result<WorkId> {
        let id = insert(
            &self.conn,
            "INSERT INTO sniffer_subscribers (computer_id, router_id) VALUES (?1, ?2)",
            params![computer.0, router.map(|router| router.0)],
        )
        .await?;

        Ok(SNIFFER_BASE + id.cast_unsigned())
    }

    /// Returns sniffers on networks belonging to `routers`, in start order.
    pub(crate) async fn sniffers_on(&self, routers: &[RouterId]) -> Result<Vec<WorkId>> {
        let mut sniffers = Vec::new();

        for router in routers {
            let rows: Vec<i64> = super::sql::fetch_column(
                &self.conn,
                "SELECT id FROM sniffer_subscribers WHERE router_id = ?1 ORDER BY id",
                params![router.0],
            )
            .await?;

            sniffers.extend(
                rows.into_iter()
                    .map(|row| SNIFFER_BASE + row.cast_unsigned()),
            );
        }

        sniffers.sort_unstable();
        sniffers.dedup();

        Ok(sniffers)
    }

    /// Returns sniffers running directly on `computer`, in start order.
    pub(crate) async fn active_sniffers_on(&self, computer: ComputerId) -> Result<Vec<WorkId>> {
        let rows: Vec<i64> = super::sql::fetch_column(
            &self.conn,
            "SELECT id FROM sniffer_subscribers WHERE computer_id = ?1 ORDER BY id",
            params![computer.0],
        )
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| SNIFFER_BASE + row.cast_unsigned())
            .collect())
    }

    /// Returns the device running sniffer `id`.
    pub(crate) async fn sniffer_computer(&self, id: WorkId) -> Result<Option<ComputerId>> {
        let Some(row) = row_of(id, SNIFFER_BASE) else {
            return Ok(None);
        };

        Ok(fetch_value::<u32>(
            &self.conn,
            "SELECT computer_id FROM sniffer_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
        .map(ComputerId))
    }

    /// Ends sniffer `id` and returns whether it existed.
    pub(crate) async fn remove_sniffer(&self, id: WorkId) -> Result<bool> {
        let Some(row) = row_of(id, SNIFFER_BASE) else {
            return Ok(false);
        };

        Ok(execute(
            &self.conn,
            "DELETE FROM sniffer_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
            > 0)
    }

    /// Starts a flood against `computer` from `origin`. Returns its work ID and whether it counts
    /// toward reboot; only the first origin flood counts (`FloodSystem.AddFlood`).
    pub(crate) async fn insert_flood(
        &self,
        computer: ComputerId,
        origin: &str,
    ) -> Result<(WorkId, bool)> {
        let counted = fetch_value::<i64>(
            &self.conn,
            "SELECT id FROM flood_subscribers WHERE computer_id = ?1 AND origin = ?2 AND counted = 1",
            params![computer.0, origin],
        )
        .await?
        .is_none();
        let id = insert(
            &self.conn,
            "INSERT INTO flood_subscribers (computer_id, origin, counted) VALUES (?1, ?2, ?3)",
            params![computer.0, origin, counted],
        )
        .await?;

        Ok((FLOOD_BASE + id.cast_unsigned(), counted))
    }

    /// Counts floods contributing to `computer` reboot (`FloodSystem.computerFloods`).
    pub(crate) async fn flood_count(&self, computer: ComputerId) -> Result<u32> {
        Ok(fetch_value::<u32>(
            &self.conn,
            "SELECT COUNT(*) FROM flood_subscribers WHERE computer_id = ?1 AND counted = 1",
            params![computer.0],
        )
        .await?
        .unwrap_or(0))
    }

    /// Clears all `computer` flood records while scripts continue waiting
    /// (`FloodSystem.ClearComputerFloods`).
    pub(crate) async fn clear_floods(&self, computer: ComputerId) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM flood_subscribers WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?;

        Ok(())
    }

    /// Ends flood `id` and its counted computer/origin entry (`FloodSystem.RemoveFlood`). Returns
    /// whether it existed.
    pub(crate) async fn remove_flood(&self, id: WorkId) -> Result<bool> {
        let Some(row) = row_of(id, FLOOD_BASE) else {
            return Ok(false);
        };
        let Some((computer, origin)) = super::sql::fetch_opt::<(u32, String)>(
            &self.conn,
            "SELECT computer_id, origin FROM flood_subscribers WHERE id = ?1",
            params![row],
        )
        .await?
        else {
            return Ok(false);
        };

        execute(
            &self.conn,
            "DELETE FROM flood_subscribers WHERE id = ?1 \
             OR (computer_id = ?2 AND origin = ?3 AND counted = 1)",
            params![row, computer, origin.as_str()],
        )
        .await?;

        Ok(true)
    }
}
