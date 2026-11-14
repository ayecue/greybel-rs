//! Persistent `world_meta` state: seed, random generator, clock, and player.

use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_opt};
use crate::{ComputerId, PersonId, Result};

/// Database representation of `world_meta`.
pub struct WorldMeta {
    pub seed: u64,
    pub rng_state: u64,
    pub player: ComputerId,
    pub player_person: PersonId,
    pub elapsed_seconds: u64,
    pub clock_carry: u32,
    pub world_secret: Vec<u8>,
}

impl FromRow for WorldMeta {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            seed: row.get::<i64>("seed")?.cast_unsigned(),
            rng_state: row.get::<i64>("rng_state")?.cast_unsigned(),
            player: ComputerId(row.get("player_computer_id")?),
            player_person: PersonId(row.get("player_person_id")?),
            elapsed_seconds: row.get("elapsed_seconds")?,
            clock_carry: row.get("clock_carry")?,
            world_secret: row.get("world_secret")?,
        })
    }
}

/// Queries of the tables owned by this area.
pub(crate) struct WorldMetaRepository {
    db: Database,
}

impl WorldMetaRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Returns the world metadata row, or `None` if the database contains no world.
    pub async fn world_meta(&self) -> Result<Option<WorldMeta>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT seed, rng_state, player_computer_id, player_person_id, elapsed_seconds, clock_carry, \
             world_secret FROM world_meta WHERE id = 0",
            (),
        )
        .await?)
    }

    /// Updates the world metadata row.
    pub async fn save_world_meta(&self, meta: &WorldMeta) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO world_meta (id, seed, rng_state, player_computer_id, player_person_id, \
             elapsed_seconds, clock_carry, world_secret) \
             VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                meta.seed.cast_signed(),
                meta.rng_state.cast_signed(),
                meta.player.0,
                meta.player_person.0,
                meta.elapsed_seconds.cast_signed(),
                meta.clock_carry,
                meta.world_secret.clone(),
            ],
        )
        .await?;

        Ok(())
    }
}
