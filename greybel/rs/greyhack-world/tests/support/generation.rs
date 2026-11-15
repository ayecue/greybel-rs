use greyhack_behaviour::{NetworkKind, WorldSeed, kind_for_ip};
use greyhack_world::World;
use greyhack_world_core::{ComputerId, RouterId};
use greyhack_world_creation::NetworkOptions;

/// A public address the world gives network kind `kind` (`IpGenerator.GetNetworkType`).
pub fn address_for_kind(seed: &WorldSeed, kind: NetworkKind) -> String {
    (1..=u16::MAX)
        .map(|n| format!("8.8.{}.{}", n >> 8, n & 0xff))
        .find(|address| kind_for_ip(seed, address) == kind)
        .unwrap_or_else(|| panic!("no test address for {kind:?}"))
}

pub async fn add_typed_network(
    world: &mut World,
    kind: NetworkKind,
    domain: &str,
) -> (RouterId, ComputerId) {
    let address = address_for_kind(&world.handle.world_seed(), kind);
    let router = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions {
            public_ip: Some(address),
            domain: Some(domain.to_owned()),
            computers: Some(1),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let computer = world
        .modules
        .core
        .networks
        .topology
        .devices(router)
        .await
        .unwrap()[0];

    (router, computer)
}
