use greyhack_world::World;

pub async fn rebooting(world: &World) -> bool {
    world
        .computer(world.player())
        .await
        .unwrap()
        .unwrap()
        .is_rebooting(world.time_seconds())
}
