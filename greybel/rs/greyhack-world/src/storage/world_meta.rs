//! Persistent `world_meta` state: seed, random generator, clock, and player.

use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_opt};
use crate::{ComputerId, PersonId, Result};

/// Database representation of `world_meta`.
pub(crate) struct WorldMeta {
    pub(crate) seed: u64,
    pub(crate) rng_state: u64,
    pub(crate) player: ComputerId,
    pub(crate) player_person: PersonId,
    pub(crate) elapsed_seconds: u64,
    pub(crate) clock_carry: u32,
    pub(crate) traffic_cooldown_until: Option<u64>,
    pub(crate) world_secret: Vec<u8>,
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
            traffic_cooldown_until: row.get("traffic_cooldown_until")?,
            world_secret: row.get("world_secret")?,
        })
    }
}

impl Database {
    /// Returns the world metadata row, or `None` if the database contains no world.
    pub(crate) async fn world_meta(&self) -> Result<Option<WorldMeta>> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT seed, rng_state, player_computer_id, player_person_id, elapsed_seconds, clock_carry, \
             traffic_cooldown_until, world_secret FROM world_meta WHERE id = 0",
            (),
        )
        .await?)
    }

    /// Updates the world metadata row.
    pub(crate) async fn save_world_meta(&self, meta: &WorldMeta) -> Result<()> {
        execute(
            &self.conn,
            "INSERT OR REPLACE INTO world_meta (id, seed, rng_state, player_computer_id, player_person_id, \
             elapsed_seconds, clock_carry, traffic_cooldown_until, world_secret) \
             VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                meta.seed.cast_signed(),
                meta.rng_state.cast_signed(),
                meta.player.0,
                meta.player_person.0,
                meta.elapsed_seconds.cast_signed(),
                meta.clock_carry,
                meta.traffic_cooldown_until.map(u64::cast_signed),
                meta.world_secret.clone(),
            ],
        )
        .await?;

        Ok(())
    }
}
