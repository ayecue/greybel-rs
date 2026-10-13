use greyhack_behaviour::{NetworkKind, kind_for_ip};

use crate::{NetworkId, NetworkOptions, PlayerOptions, Rng, World};

/// A world with a generated network of `kind`, and that network.
pub(in crate::systems) async fn world_with(kind: NetworkKind) -> (World, NetworkId) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut rng = Rng::new(7);
    let address = loop {
        let address = rng.public_ip();

        if kind_for_ip(&world.world_seed, &address) == kind
            && world.router_at(&address).await.unwrap().is_none()
        {
            break address;
        }
    };
    let router = world
        .add_network(&NetworkOptions {
            public_ip: Some(address),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let network = world.db.router(router).await.unwrap().unwrap().network;

    (world, network)
}

/// The public address of `network`.
pub(in crate::systems) async fn public_ip(world: &World, network: NetworkId) -> String {
    world.main_router(network).await.unwrap().unwrap().public_ip
}
