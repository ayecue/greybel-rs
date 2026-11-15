use greyhack_behaviour::{NetworkKind, Rng, kind_for_ip};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::NetworkId;
use greyhack_world_creation::NetworkOptions;

/// A world with a generated network of `kind`, and that network.
pub async fn world_with(kind: NetworkKind) -> (World, NetworkId) {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut rng = Rng::new(7);
    let address = loop {
        let address = rng.public_ip();

        if kind_for_ip(&world.handle.world().world_seed, &address) == kind
            && world
                .modules
                .core
                .networks
                .routers
                .router_at(&address)
                .await
                .unwrap()
                .is_none()
        {
            break address;
        }
    };
    let router = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions {
            public_ip: Some(address),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let network = world
        .modules
        .core
        .networks
        .router(router)
        .await
        .unwrap()
        .unwrap()
        .network;

    (world, network)
}

/// The public address of `network`.
pub async fn public_ip(world: &World, network: NetworkId) -> String {
    world
        .modules
        .core
        .networks
        .topology
        .main_router(network)
        .await
        .unwrap()
        .unwrap()
        .public_ip
}
