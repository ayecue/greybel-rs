//! World-generation options and the player-world builder.

pub(crate) mod player_world;
pub(crate) mod preset;

pub(crate) use player_world::player_world;
pub use preset::PlayerOptions;

/// Configuration for world generation.
#[derive(Debug, Clone)]
pub struct WorldOptions {
    /// Seed of the world's deterministic random streams.
    pub seed: u64,
    /// Additional network count beyond the player network.
    pub networks: u32,
    /// Player identity and starting computer settings.
    pub player: PlayerOptions,
}

impl Default for WorldOptions {
    fn default() -> Self {
        Self {
            seed: 0,
            networks: 5,
            player: PlayerOptions::default(),
        }
    }
}
