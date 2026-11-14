//! Storage for coins, their price history, and pending trades.

use greyhack_world_core::Database;
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, insert};
use turso::params;

use crate::{Coin, CoinId, PriceRecord, Result};

/// Coin columns in [`Coin`] [`FromRow`] order.
macro_rules! coin_columns {
    () => {
        "id, name, creator_user, creator_password_hash, address, mining_cycle, reward, mined_coins, max_supply, \
         price, last_cycle_at"
    };
}

impl FromRow for Coin {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: CoinId(row.get("id")?),
            name: row.get("name")?,
            creator_user: row.get("creator_user")?,
            creator_password_hash: row.get("creator_password_hash")?,
            address: row.get("address")?,
            cycle_hours: row.get("mining_cycle")?,
            last_cycle_at: row.get("last_cycle_at")?,
            reward: row.get("reward")?,
            mined_coins: row.get("mined_coins")?,
            max_supply: row.get("max_supply")?,
            price: row.get("price")?,
        })
    }
}

impl FromRow for PriceRecord {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            price: row.get("price")?,
            date: row.get("date")?,
        })
    }
}

/// Queries of [`Database`] for economy coins.
/// Queries of the tables owned by this area.
pub(crate) struct CoinRepository {
    db: Database,
    computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
}

impl CoinRepository {
    pub(crate) fn new(
        db: Database,
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    ) -> Self {
        Self { db, computers }
    }

    pub(crate) async fn coin(&self, id: CoinId) -> Result<Option<Coin>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!("SELECT ", coin_columns!(), " FROM coins WHERE id = ?1"),
            params![id.0],
        )
        .await?)
    }

    pub(crate) async fn coin_by_name(&self, name: &str) -> Result<Option<Coin>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!("SELECT ", coin_columns!(), " FROM coins WHERE name = ?1"),
            params![name],
        )
        .await?)
    }

    pub(crate) async fn coins(&self) -> Result<Vec<Coin>> {
        Ok(fetch_all(
            &self.db.connection(),
            concat!("SELECT ", coin_columns!(), " FROM coins ORDER BY id"),
            (),
        )
        .await?)
    }

    pub(crate) async fn coins_of_creator(&self, creator_user: &str) -> Result<Vec<Coin>> {
        Ok(fetch_all(
            &self.db.connection(),
            concat!(
                "SELECT ",
                coin_columns!(),
                " FROM coins WHERE creator_user = ?1 ORDER BY id"
            ),
            params![creator_user],
        )
        .await?)
    }

    pub(crate) async fn insert_coin(&self, coin: &Coin, password: Option<&str>) -> Result<CoinId> {
        if let Some(password) = password {
            self.computers.register_password(password).await?;
        }

        let id = insert(
            &self.db.connection(),
            "INSERT INTO coins (name, creator_user, creator_password_hash, address, mining_cycle, reward, mined_coins, \
             max_supply, price, last_cycle_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                coin.name.as_str(),
                coin.creator_user.as_str(),
                coin.creator_password_hash.as_str(),
                coin.address.as_str(),
                coin.cycle_hours,
                coin.reward,
                coin.mined_coins,
                coin.max_supply,
                coin.price,
                coin.last_cycle_at.map(u64::cast_signed),
            ],
        )
        .await?;

        Ok(CoinId(u32::try_from(id).expect("coin ids fit u32")))
    }

    pub(crate) async fn save_coin(&self, coin: &Coin) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE coins SET name = ?2, creator_user = ?3, creator_password_hash = ?4, address = ?5, \
             mining_cycle = ?6, reward = ?7, mined_coins = ?8, max_supply = ?9, price = ?10, last_cycle_at = ?11 \
             WHERE id = ?1",
            params![
                coin.id.0,
                coin.name.as_str(),
                coin.creator_user.as_str(),
                coin.creator_password_hash.as_str(),
                coin.address.as_str(),
                coin.cycle_hours,
                coin.reward,
                coin.mined_coins,
                coin.max_supply,
                coin.price,
                coin.last_cycle_at.map(u64::cast_signed),
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn remove_coin(&self, id: CoinId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM coins WHERE id = ?1",
            params![id.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn price_history(&self, coin: CoinId) -> Result<Vec<PriceRecord>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT price, date FROM coin_prices WHERE coin_id = ?1 ORDER BY id",
            params![coin.0],
        )
        .await?)
    }

    pub(crate) async fn insert_price(&self, coin: CoinId, record: &PriceRecord) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO coin_prices (coin_id, price, date) VALUES (?1, ?2, ?3)",
            params![coin.0, record.price, record.date.as_str()],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn remove_oldest_price(&self, coin: CoinId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM coin_prices WHERE id = \
             (SELECT id FROM coin_prices WHERE coin_id = ?1 ORDER BY id LIMIT 1)",
            params![coin.0],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn pending_trades(&self, coin: CoinId) -> Result<(i64, i64)> {
        Ok(fetch_opt::<(i64, i64)>(
            &self.db.connection(),
            "SELECT value, amount FROM coin_pending_trades WHERE coin_id = ?1",
            params![coin.0],
        )
        .await?
        .unwrap_or((0, 0)))
    }

    pub(crate) async fn record_trade(&self, coin: CoinId, price: i64, amount: i64) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO coin_pending_trades (coin_id, value, amount) VALUES (?1, ?2, ?3) \
             ON CONFLICT (coin_id) DO UPDATE SET value = value + ?2, amount = amount + ?3",
            params![coin.0, price.saturating_mul(amount), amount],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn clear_pending_trades(&self, coin: CoinId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM coin_pending_trades WHERE coin_id = ?1",
            params![coin.0],
        )
        .await?;

        Ok(())
    }
}
