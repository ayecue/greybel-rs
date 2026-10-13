use greyhack_behaviour::{NetworkKind, SECONDS_PER_DAY, kind_for_ip};

use crate::{ComputerId, NetworkId, NetworkOptions, PlayerOptions, Rng, World};

pub(in crate::systems) const DAY: u64 = SECONDS_PER_DAY;
/// Experience of a level-5 guild, which may invest in one company.
pub(in crate::systems) const LEVEL_FIVE: u32 = 275 * 25;

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

/// Shares a bank company issues.
pub(in crate::systems) const BANK_SHARES: u32 = 1_000_000;

/// A bank network whose administrator is logged in, with its public address, administrator experience and a computer.
pub(in crate::systems) async fn bank() -> (World, NetworkId, String, u32, ComputerId) {
    let (mut world, network) = world_with(NetworkKind::Bank).await;
    let address = public_ip(&world, network).await;
    let admin = world
        .db
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let exp = u32::from(
        world
            .db
            .network_admin(network)
            .await
            .unwrap()
            .unwrap()
            .pc_experience,
    );
    let router = world.router_at(&address).await.unwrap().unwrap();

    world.set_npc_login(admin, true).await.unwrap();
    world
        .set_npc_forced_login(admin, Some(world.time_seconds() + 30 * DAY))
        .await
        .unwrap();

    (world, network, address, exp, router.id.computer())
}
