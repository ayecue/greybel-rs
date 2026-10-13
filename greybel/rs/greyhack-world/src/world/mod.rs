//! World database, in-memory `world_meta` state (random generator and clock), and program-lifetime
//! work.

mod events;
mod work;

pub use events::WorldEvent;
use greyhack_behaviour::{WORLD_SECRET_LENGTH, WorldSeed, fnv1a_64};
pub use work::{MineJob, WorkEnd, WorkResult};
pub(crate) use work::{Work, WorkId};

use crate::storage::world_meta::WorldMeta;
use crate::{
    ComputerId, Database, LocatedVehicle, NetworkOptions, PersonId, PlayerOptions, Result, Rng,
    WorldOptions, generation,
};

/// World state. Entities are database rows; the struct stores seed, random generator, clock, player,
/// and transient program work. `world_meta` is written on cycle commit.
#[derive(Debug)]
pub struct World {
    pub(crate) db: Database,
    pub seed: u64,
    pub(crate) rng: Rng,
    pub(crate) player: ComputerId,
    pub(crate) player_person: PersonId,
    pub(crate) time_seconds: u64,
    /// Fractional world time accumulated by clock ticks but not yet counted, in `1 / rate`
    /// tick-second units. Always below the tick rate.
    pub(crate) clock_carry: u32,
    pub(crate) traffic_cooldown_until: Option<u64>,
    /// World secret (`WorldSeed`) derived from its seed, or copied from Grey Hack for mirrored
    /// worlds.
    pub(crate) world_seed: WorldSeed,
    /// Last car found by `locate_vehicle`, used by accidents. Grey Hack stores this per player
    /// session, so it is not persisted (`PlayerHelperServer.npcInfoCar`).
    pub(crate) located_vehicle: Option<LocatedVehicle>,
    /// Pending notifications other than completed work, accumulated since [`World::take_events`].
    pub(crate) events: Vec<WorldEvent>,
    pub(crate) work: Work,
    /// Computers with processes ended since [`World::take_changed_computers`].
    pub(crate) process_ends: std::collections::BTreeSet<ComputerId>,
    /// Whether stocks or guilds changed since the Crown ranking was last evaluated
    /// (`Guilds.crownRankingDirty`), true when the world opens. Not persisted.
    pub(crate) crown_ranking_dirty: bool,
    /// World second the Crown ranking is evaluated again (`Guilds.nextCrownRankingEvaluation`).
    pub(crate) next_crown_evaluation: u64,
}

impl World {
    /// Creates a world on `db` for `player` at `time_seconds`. Generation creates entity rows; the
    /// first [`Self::commit`] writes `world_meta`.
    pub(crate) fn on(db: Database, seed: u64, player: ComputerId, time_seconds: u64) -> Self {
        Self {
            db,
            seed,
            rng: Rng::new(seed),
            player,
            player_person: PersonId(0),
            time_seconds,
            clock_carry: 0,
            traffic_cooldown_until: None,
            world_seed: WorldSeed::new(secret_of(seed)),
            located_vehicle: None,
            events: Vec::new(),
            work: Work::default(),
            process_ends: std::collections::BTreeSet::new(),
            crown_ranking_dirty: true,
            next_crown_evaluation: 0,
        }
    }

    /// Creates an in-memory world containing only the player.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn new(player: &PlayerOptions) -> Result<Self> {
        Self::generate(&WorldOptions {
            seed: 0,
            networks: 0,
            player: player.clone(),
        })
        .await
    }

    /// Creates a player-only in-memory world seeded by the FNV-1a hash of `seed`. Identical text
    /// produces identical worlds.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn from_seed(seed: &str, player: &PlayerOptions) -> Result<Self> {
        Self::generate(&WorldOptions {
            seed: fnv1a_64(seed),
            networks: 0,
            player: player.clone(),
        })
        .await
    }

    /// Generates the player network and `options.networks` additional networks from `options.seed`
    /// in a new in-memory database.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn generate(options: &WorldOptions) -> Result<Self> {
        Self::generate_in(options, Database::in_memory().await?).await
    }

    /// Generates a world into `db` using one transaction per network.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub(crate) async fn generate_in(options: &WorldOptions, db: Database) -> Result<Self> {
        let mut world = generation::player_world(options, db).await?;

        for _ in 0..options.networks {
            world.begin().await?;
            world.add_network(&NetworkOptions::default()).await?;
            world.commit().await?;
        }

        Ok(world)
    }

    /// Replaces `db` contents with a copy of this world.
    ///
    /// # Errors
    ///
    /// Returns any statement error from either database. `db` remains unchanged.
    pub async fn save_into(&mut self, db: &Database) -> Result<()> {
        self.commit().await?;

        Ok(self.db.copy_into(db).await?)
    }

    /// Loads a world from `db`.
    ///
    /// # Errors
    ///
    /// Returns any database read error, or an error if no world is stored.
    pub async fn load(db: &Database) -> Result<Self> {
        let mut world = Self::on(db.clone(), 0, ComputerId(0), 0);

        world.read_meta().await?;

        Ok(world)
    }

    /// Returns computers whose processes were killed, rebooted, or removed since the last call.
    pub fn take_changed_computers(&mut self) -> Vec<ComputerId> {
        std::mem::take(&mut self.process_ends).into_iter().collect()
    }

    /// Returns the world database.
    #[must_use]
    pub fn database(&self) -> &Database {
        &self.db
    }

    /// Returns the player computer.
    #[must_use]
    pub fn player(&self) -> ComputerId {
        self.player
    }

    /// Returns the player.
    #[must_use]
    pub fn player_person(&self) -> PersonId {
        self.player_person
    }

    /// Returns whether `computer` belongs to the player.
    #[must_use]
    pub fn is_player(&self, computer: ComputerId) -> bool {
        self.player == computer
    }

    /// Returns the date displayed by `current_date`.
    #[must_use]
    pub fn date(&self) -> String {
        greyhack_behaviour::format_date(self.time_seconds)
    }

    /// Returns the world clock in seconds.
    #[must_use]
    pub fn time_seconds(&self) -> u64 {
        self.time_seconds
    }

    /// Begins a per-cycle database transaction, dropping any earlier uncommitted transaction. The
    /// next [`Self::commit`] commits it.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot open the transaction.
    pub async fn begin(&self) -> Result<()> {
        Ok(self.db.begin().await?)
    }

    /// Writes world metadata (random generator, clock, cooldown) and commits the open transaction,
    /// or uses a new transaction if none is open.
    ///
    /// # Errors
    ///
    /// Returns any database write or commit error and drops the transaction. In-memory state
    /// remains unchanged for the next commit.
    pub async fn commit(&mut self) -> Result<()> {
        let own = !self.db.in_transaction()?;

        if own {
            self.db.begin().await?;
        }

        let commit = async {
            self.write_meta().await?;
            self.db.commit().await?;

            Ok(())
        }
        .await;

        if commit.is_err() {
            self.db.rollback().await?;
        }

        commit
    }

    /// Rolls back the open transaction. In-memory clock and random-generator state remain, so
    /// clock-only cycles can commit later.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot drop the transaction.
    pub async fn rollback(&self) -> Result<()> {
        Ok(self.db.rollback().await?)
    }

    async fn read_meta(&mut self) -> Result<()> {
        let meta = self.db.world_meta().await?.ok_or_else(|| {
            turso::Error::ConversionFailure("the database has no world".to_owned())
        })?;

        self.seed = meta.seed;
        self.rng = Rng::new(meta.rng_state);
        self.player = meta.player;
        self.player_person = meta.player_person;
        self.time_seconds = meta.elapsed_seconds;
        self.clock_carry = meta.clock_carry;
        self.traffic_cooldown_until = meta.traffic_cooldown_until;
        self.world_seed = WorldSeed::new(meta.world_secret);

        Ok(())
    }

    async fn write_meta(&self) -> Result<()> {
        self.db
            .save_world_meta(&WorldMeta {
                seed: self.seed,
                rng_state: self.rng.state(),
                player: self.player,
                player_person: self.player_person,
                elapsed_seconds: self.time_seconds,
                clock_carry: self.clock_carry,
                traffic_cooldown_until: self.traffic_cooldown_until,
                world_secret: self.world_seed.secret().to_vec(),
            })
            .await
    }

    /// Returns the world secret (`WorldSeed`).
    #[must_use]
    pub fn world_seed(&self) -> &WorldSeed {
        &self.world_seed
    }

    /// Replaces the world secret from mirrored Grey Hack state (`InfoGen.WorldSecret`). Existing
    /// license plates retain their original secret.
    pub fn set_world_secret(&mut self, secret: Vec<u8>) {
        self.world_seed = WorldSeed::new(secret);
    }
}

/// Derives [`WORLD_SECRET_LENGTH`] secret bytes from `seed`, matching Grey Hack's draw count.
fn secret_of(seed: u64) -> Vec<u8> {
    let mut rng = Rng::new(seed ^ SECRET_SALT);

    (0..WORLD_SECRET_LENGTH)
        .map(|_| rng.range(0, 255) as u8)
        .collect()
}

/// Separates secret generation from the world random sequence.
const SECRET_SALT: u64 = 0x5745_524c_4453_4543;
