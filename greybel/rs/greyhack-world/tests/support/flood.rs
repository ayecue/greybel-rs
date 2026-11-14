use greyhack_world::World;

pub async fn rebooting(world: &World) -> bool {
    world
        .modules
        .core
        .networks
        .topology
        .computer(world.handle.player())
        .await
        .unwrap()
        .unwrap()
        .is_rebooting(world.handle.time_seconds())
}
