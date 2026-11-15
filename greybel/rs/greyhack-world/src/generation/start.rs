//! Constructors of an assembled world.

use greyhack_behaviour::fnv1a_64;
use greyhack_world_core::{CoreModule, Database, Result};
use greyhack_world_creation::NetworkOptions;

use super::{PlayerOptions, WorldOptions, player_world};
use crate::World;
use crate::storage::DatabaseOpen;

impl World {
    /// Creates an in-memory world containing only the player.
    pub async fn new(player: &PlayerOptions) -> Result<Self> {
        Self::generate(&WorldOptions {
            seed: 0,
            networks: 0,
            player: player.clone(),
        })
        .await
    }

    /// Creates a player-only world seeded by the FNV-1a hash of `seed`.
    pub async fn from_seed(seed: &str, player: &PlayerOptions) -> Result<Self> {
        Self::generate(&WorldOptions {
            seed: fnv1a_64(seed),
            networks: 0,
            player: player.clone(),
        })
        .await
    }

    /// Generates the player network and `options.networks` additional networks.
    pub async fn generate(options: &WorldOptions) -> Result<Self> {
        Self::generate_in(options, Database::in_memory().await?).await
    }

    /// Generates a world into `db` using one transaction per network.
    pub async fn generate_in(options: &WorldOptions, db: Database) -> Result<Self> {
        let world = player_world(options, db).await?;

        for _ in 0..options.networks {
            world.begin().await?;
            world
                .modules
                .creation
                .network_generation
                .add_network(&NetworkOptions::default())
                .await?;
            world.commit().await?;
        }

        Ok(world)
    }

    /// Loads a world and every service from `db`.
    pub async fn load(db: &Database) -> Result<Self> {
        let (handle, core, _) = CoreModule::load(db).await?;
        let world = Self::from_core(handle, core);

        world.modules.core.load_state().await?;
        world.modules.libraries.check.ensure().await?;
        world.modules.economy.daily_upkeep.ensure().await?;

        Ok(world)
    }
}
