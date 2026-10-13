use greyhack_world::{Router, World};

use super::{routers, world_options};

/// A generated world with its networks' main routers.
pub async fn world() -> (World, Vec<Router>) {
    let world = World::generate(&world_options(5, 12)).await.unwrap();
    let mains = routers(&world).await;

    (world, mains)
}
