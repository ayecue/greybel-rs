use greyhack_behaviour::WorldSeed;

pub fn seed() -> WorldSeed {
    WorldSeed::new((0..32).collect())
}
