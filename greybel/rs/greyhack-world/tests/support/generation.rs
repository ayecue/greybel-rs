use greyhack_behaviour::{NetworkKind, WorldSeed, kind_for_ip};
use greyhack_world::{ComputerId, NetworkOptions, RouterId, World};

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
    let address = address_for_kind(world.world_seed(), kind);
    let router = world
        .add_network(&NetworkOptions {
            public_ip: Some(address),
            domain: Some(domain.to_owned()),
            computers: Some(1),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let computer = world.devices(router).await.unwrap()[0];

    (router, computer)
}
