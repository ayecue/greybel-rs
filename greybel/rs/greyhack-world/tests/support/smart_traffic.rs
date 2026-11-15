use greyhack_behaviour::SECONDS_PER_DAY;
use greyhack_world::World;
use greyhack_world_npcs::Npc;

/// The first world second of the coming week at which `npc` drives to or from work, or doesn't.
pub fn commute(world: &World, npc: &Npc, commuting: bool) -> u64 {
    let schedule = npc.schedule.clone();
    let start = world.handle.time_seconds();

    (start..start + 7 * SECONDS_PER_DAY)
        .step_by(60)
        .find(|at| schedule.is_commuting(*at) == commuting)
        .expect("a week has commutes and other times")
}
