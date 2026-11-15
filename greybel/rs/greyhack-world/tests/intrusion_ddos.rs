use greyhack_behaviour::{SECONDS_PER_DAY, flood_reboot_threshold};
use greyhack_world_intrusion::{Flood, FloodWarning};
use support::{company, guild_owning};

use crate::support::intrusion_ddos as support;

#[tokio::test]
async fn should_warn_and_not_flood_a_network_under_crownshield() {
    let (mut world, address, router) = company().await;

    guild_owning(&mut world, &address).await;
    world.tick(0).await.unwrap();
    world.tick(15 * SECONDS_PER_DAY).await.unwrap();
    world
        .modules
        .guilds
        .guilds
        .hire_crown_shield("Owls", &address)
        .await
        .unwrap()
        .unwrap();

    let attempt = world
        .modules
        .intrusion
        .floods
        .flood_from(router.computer(), world.handle.player())
        .await
        .unwrap();

    assert_eq!(attempt.flood, Flood::Shield);
    assert_eq!(attempt.warning, Some(FloodWarning::CrownShield));
    assert_eq!(attempt.address, address);
}

#[tokio::test]
async fn should_lower_the_ranking_of_a_guild_whose_company_a_flood_took_down() {
    let (mut world, address, router) = company().await;

    guild_owning(&mut world, &address).await;

    let before = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;
    let net_speed = world
        .modules
        .core
        .networks
        .settings
        .net_speed(router)
        .await
        .unwrap()
        .unwrap();
    let network = world
        .modules
        .core
        .networks
        .routers
        .router(router)
        .await
        .unwrap()
        .unwrap()
        .network;
    let threshold = flood_reboot_threshold(
        net_speed,
        world
            .modules
            .npcs
            .jobs
            .has_super_admin(network)
            .await
            .unwrap(),
    );

    for origin in 1..threshold {
        world
            .modules
            .intrusion
            .floods
            .flood(router.computer(), &format!("9.9.9.{origin}"), threshold)
            .await
            .unwrap();
    }

    let attempt = world
        .modules
        .intrusion
        .floods
        .flood_from(router.computer(), world.handle.player())
        .await
        .unwrap();
    let after = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    assert_eq!(attempt.flood, Flood::Reboot);
    assert!(after < before, "{before} -> {after}");
}
