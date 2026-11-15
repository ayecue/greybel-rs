//! Queries of economy-owned routine data.

use greyhack_world_core::{CoinId, Database, Result};
use greyhack_world_effects::RoutineId;
use greyhack_world_storage::sql::fetch_value;
use turso::params;

/// Queries of economy routine rows.
pub(crate) struct RoutineRepository {
    db: Database,
}

impl RoutineRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn coin_round(&self, coin: CoinId) -> Result<Option<RoutineId>> {
        Ok(fetch_value::<i64>(
            &self.db.connection(),
            "SELECT routine_id FROM routine_mining_rounds WHERE coin_id = ?1",
            params![coin.0],
        )
        .await?
        .map(|id| RoutineId::new(id.cast_unsigned())))
    }
}
