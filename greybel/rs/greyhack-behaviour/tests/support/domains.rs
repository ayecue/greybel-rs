use greyhack_behaviour::WorldSeed;

/// A world seed of 32 times `byte`.
pub fn seed(byte: u8) -> WorldSeed {
    WorldSeed::new(vec![byte; 32])
}
