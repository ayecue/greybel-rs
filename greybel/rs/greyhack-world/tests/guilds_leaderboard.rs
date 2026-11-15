use greyhack_behaviour::JobRole;
use greyhack_world_guilds::GuildDraft;

use crate::support::guilds_records::{
    DAY, LEVEL_FIVE, SHARES, guild, has_crown, network_of, own, world_with_companies,
};

#[tokio::test]
async fn should_give_the_crown_after_fifteen_days_of_leading() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;

    guild(&mut world, "Owls", "owls-bank", true).await;
    own(&mut world, &addresses[0], "owls-bank").await;
    world.tick(0).await.unwrap();

    // Act
    world.tick(15 * DAY - 1).await.unwrap();

    let early = has_crown(&world, "Owls").await;

    world.tick(DAY).await.unwrap();

    // Assert
    assert!(!early);
    assert!(has_crown(&world, "Owls").await);
}

#[tokio::test]
async fn should_not_count_a_company_beyond_the_guilds_licences() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;

    world
        .modules
        .guilds
        .guilds
        .create_guild(&GuildDraft {
            name: "Owls".to_owned(),
            bank_account: "owls-bank".to_owned(),
            total_exp: LEVEL_FIVE,
            investment_perks: 0,
            admin_restore_perks: 0,
            player_member: false,
        })
        .await
        .unwrap()
        .unwrap();
    own(&mut world, &addresses[0], "owls-bank").await;

    // Act
    let ranking = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap();

    // Assert
    assert_eq!(ranking.len(), 1);
    assert_eq!(ranking[0].total_points, 0);
}

#[tokio::test]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Database.GetGuildRanking applies the DDoS factor with C# floats and Math.Round"
)]
async fn should_lower_a_guilds_points_by_a_fifth_for_a_day_after_a_ddos() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;

    guild(&mut world, "Owls", "owls-bank", false).await;
    own(&mut world, &addresses[0], "owls-bank").await;

    let before = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    // Act
    let applied = world
        .modules
        .guilds
        .guilds
        .apply_ddos_leaderboard_penalty(&addresses[0])
        .await
        .unwrap();
    let during = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    world.tick(DAY + 1).await.unwrap();

    // Assert
    assert!(applied);
    assert_eq!(during, (before as f64 * 0.8).round_ties_even() as i64);
    assert_eq!(
        world
            .modules
            .guilds
            .guilds
            .guild_leaderboard()
            .await
            .unwrap()[0]
            .total_points,
        before
    );
}

#[tokio::test]
async fn should_not_penalize_a_ddos_on_a_company_no_guild_owns() {
    // Arrange
    let (world, addresses) = world_with_companies(1).await;

    world
        .modules
        .guilds
        .guilds
        .set_company_shares(&addresses[0], "someone", SHARES)
        .await
        .unwrap();

    // Act
    let applied = world
        .modules
        .guilds
        .guilds
        .apply_ddos_leaderboard_penalty(&addresses[0])
        .await
        .unwrap();

    // Assert
    assert!(!applied);
}

#[tokio::test]
async fn should_halve_a_guilds_points_while_a_companys_administrator_is_missing() {
    // Arrange
    let (mut world, addresses) = world_with_companies(1).await;
    let network = network_of(&world, &addresses[0]).await;
    let admin = world
        .modules
        .npcs
        .npcs
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let user = world
        .modules
        .npcs
        .npcs
        .npc(admin)
        .await
        .unwrap()
        .unwrap()
        .user;

    guild(&mut world, "Owls", "owls-bank", false).await;
    own(&mut world, &addresses[0], "owls-bank").await;

    let before = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    // Act
    world
        .modules
        .npcs
        .jobs
        .fire_npc(&addresses[0], &user)
        .await
        .unwrap();

    let missing = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    world
        .modules
        .npcs
        .jobs
        .hire_npc(&addresses[0], JobRole::Admin)
        .await
        .unwrap();

    let hired = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    world.tick(5 * DAY).await.unwrap();

    let recovering = world
        .modules
        .guilds
        .guilds
        .guild_leaderboard()
        .await
        .unwrap()[0]
        .total_points;

    world.tick(DAY).await.unwrap();

    // Assert
    assert!(missing < before);
    assert_eq!(hired, missing);
    assert!(recovering >= hired);
    assert!(
        world
            .modules
            .guilds
            .guilds
            .guild_leaderboard()
            .await
            .unwrap()[0]
            .total_points
            >= recovering
    );
}
