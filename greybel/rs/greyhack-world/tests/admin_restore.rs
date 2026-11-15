use greyhack_world_guilds::{AdminRestoreFailure, GuildDraft, GuildLayout, GuildsLayout};
use greyhack_world_libraries::LibraryReportFailure;

use crate::support::admin_restore::{LEVEL_EIGHT, LEVEL_FOUR, company, guild, versions};
use crate::support::library_updates::{SSH, world};

/// A guild turns Admin Restore off for as many companies as its perks and its level allow, and a company it already
/// turned it off for stays off (`Guild.SetAdminRestoreEnabled`).
#[tokio::test]
async fn should_turn_admin_restore_off_for_no_more_companies_than_the_perks_allow() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_EIGHT, 1).await;

    let (first, _) = company(&mut world, "owls-bank").await;
    let (second, _) = company(&mut world, "owls-bank").await;

    // Act
    let off = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &first, false)
        .await
        .unwrap();
    let again = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &first, false)
        .await
        .unwrap();
    let beyond = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &second, false)
        .await
        .unwrap();
    let on = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &first, true)
        .await
        .unwrap();
    let after = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &second, false)
        .await
        .unwrap();

    // Assert
    assert_eq!(off, Ok(()));
    assert_eq!(again, Ok(()));
    assert_eq!(beyond, Err(AdminRestoreFailure::OverLimit));
    assert_eq!(on, Ok(()));
    assert_eq!(after, Ok(()));
    assert!(
        !world
            .modules
            .guilds
            .guilds
            .is_admin_restore_disabled(&first)
            .await
            .unwrap()
    );
    assert!(
        world
            .modules
            .guilds
            .guilds
            .is_admin_restore_disabled(&second)
            .await
            .unwrap()
    );
}

/// The perks are capped at five and the level caps them again (`GuildLevelRules.GetMaxAdminRestoreCompanies`).
#[tokio::test]
async fn should_not_turn_admin_restore_off_below_the_level_that_may_invest() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_FOUR, 5).await;
    guild(&mut world, "Hawks", "hawks-bank", LEVEL_EIGHT, 0).await;

    let (owls, _) = company(&mut world, "owls-bank").await;
    let (hawks, _) = company(&mut world, "hawks-bank").await;

    // Act
    let low_level = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &owls, false)
        .await
        .unwrap();
    let no_perks = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Hawks", &hawks, false)
        .await
        .unwrap();

    // Assert
    assert_eq!(low_level, Err(AdminRestoreFailure::OverLimit));
    assert_eq!(no_perks, Err(AdminRestoreFailure::OverLimit));
}

/// The checks run in the game's order: the company, then its majority owner, then the limit
/// (`NetworkHelperServer.AdminCompanySetAdminRestoreServerRpc`).
#[tokio::test]
async fn should_refuse_a_company_the_guild_does_not_own() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_EIGHT, 0).await;

    let (foreign, _) = company(&mut world, "someone").await;

    // Act
    let nobody = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Nobody", &foreign, false)
        .await
        .unwrap();
    let invalid = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", "not an address", false)
        .await
        .unwrap();
    let missing = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", "1.2.3.4", false)
        .await
        .unwrap();
    let foreign_company = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &foreign, false)
        .await
        .unwrap();

    // Assert
    assert_eq!(
        nobody,
        Err(AdminRestoreFailure::UnknownGuild("Nobody".to_owned()))
    );
    assert_eq!(invalid, Err(AdminRestoreFailure::InvalidCompany));
    assert_eq!(missing, Err(AdminRestoreFailure::UnknownCompany));
    assert_eq!(foreign_company, Err(AdminRestoreFailure::NotGuildCompany));
}

/// Turning it on for a company that never had it off succeeds, and changes nothing.
#[tokio::test]
async fn should_turn_admin_restore_on_for_a_company_that_has_it_on() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_FOUR, 0).await;

    let (owned, _) = company(&mut world, "owls-bank").await;

    // Act
    let on = world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &owned, true)
        .await
        .unwrap();

    // Assert
    assert_eq!(on, Ok(()));
    assert!(
        !world
            .modules
            .guilds
            .guilds
            .is_admin_restore_disabled(&owned)
            .await
            .unwrap()
    );
}

/// A guild that lost the company no longer has it off (`Guild.RemoveInvestedCompany`).
#[tokio::test]
async fn should_forget_the_switch_of_a_company_the_guild_sold() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_EIGHT, 1).await;

    let (owned, _) = company(&mut world, "owls-bank").await;

    world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &owned, false)
        .await
        .unwrap()
        .unwrap();

    // Act
    world
        .modules
        .guilds
        .guilds
        .set_company_shares(&owned, "owls-bank", 0)
        .await
        .unwrap();
    world
        .modules
        .guilds
        .guilds
        .set_company_shares(&owned, "owls-bank", 2_000_000)
        .await
        .unwrap();

    // Assert
    assert!(
        !world
            .modules
            .guilds
            .guilds
            .is_admin_restore_disabled(&owned)
            .await
            .unwrap()
    );
}

/// An administrator whose company's guild turned Admin Restore off does not update, and nothing is stored: the
/// library stays, and a report once the guild turns it on again works (`Persona.UpdateReportedNetworks`).
#[tokio::test]
async fn should_skip_the_library_update_while_the_guild_has_admin_restore_off() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_EIGHT, 1).await;

    let (owned, computers) = company(&mut world, "owls-bank").await;
    let before = versions(&world, &computers).await;

    world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &owned, false)
        .await
        .unwrap()
        .unwrap();

    // Act
    let skipped = world
        .modules
        .libraries
        .updates
        .report_library_update(&owned, SSH)
        .await
        .unwrap();
    let skipped_unknown = world
        .modules
        .libraries
        .updates
        .report_library_update(&owned, "libnothing.so")
        .await
        .unwrap();
    let kept = versions(&world, &computers).await;

    world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &owned, true)
        .await
        .unwrap()
        .unwrap();

    let reported = world
        .modules
        .libraries
        .updates
        .report_library_update(&owned, SSH)
        .await
        .unwrap();
    let after = versions(&world, &computers).await;

    // Assert
    assert_eq!(skipped, Some(Err(LibraryReportFailure::AdminRestoreOff)));
    assert_eq!(
        skipped_unknown,
        Some(Err(LibraryReportFailure::AdminRestoreOff))
    );
    assert_eq!(before, kept);
    assert_eq!(reported, Some(Ok(())));
    assert_ne!(before, after);
}

/// Only the owning guild's switch counts: another guild's list, or a company nobody owns, never skips the update.
#[tokio::test]
async fn should_not_skip_the_update_of_a_company_another_guild_turned_off() {
    // Arrange
    let mut world = world().await;

    guild(&mut world, "Owls", "owls-bank", LEVEL_EIGHT, 1).await;

    let (owned, _) = company(&mut world, "owls-bank").await;
    let (foreign, _) = company(&mut world, "someone").await;

    world
        .modules
        .guilds
        .guilds
        .set_admin_restore("Owls", &owned, false)
        .await
        .unwrap()
        .unwrap();

    // Act
    let skipped = world
        .modules
        .libraries
        .updates
        .report_library_update(&foreign, SSH)
        .await
        .unwrap();

    // Assert
    assert_eq!(skipped, Some(Ok(())));
}

/// A mirrored world gets each guild's perks and the companies it turned Admin Restore off for.
#[tokio::test]
async fn should_mirror_the_admin_restore_state_of_the_guilds() {
    // Arrange
    let mut world = world().await;
    let (owned, computers) = company(&mut world, "owls-bank").await;
    let before = versions(&world, &computers).await;
    let layout = GuildsLayout {
        guilds: vec![GuildLayout {
            draft: GuildDraft {
                name: "Owls".to_owned(),
                bank_account: "owls-bank".to_owned(),
                total_exp: LEVEL_EIGHT,
                investment_perks: 1,
                admin_restore_perks: 1,
                player_member: false,
            },
            admin_restore_off: vec![owned.clone()],
            has_crown: false,
            crown_shield_public_ip: String::new(),
        }],
        ..GuildsLayout::default()
    };

    // Act
    world
        .modules
        .guilds
        .guilds
        .mirror_guilds(&layout)
        .await
        .unwrap();

    // Assert
    assert!(
        world
            .modules
            .guilds
            .guilds
            .is_admin_restore_disabled(&owned)
            .await
            .unwrap()
    );
    assert_eq!(
        world.modules.guilds.guilds.guilds().await.unwrap()[0].admin_restore_perks,
        1
    );
    assert_eq!(
        world
            .modules
            .libraries
            .updates
            .report_library_update(&owned, SSH)
            .await
            .unwrap(),
        Some(Err(LibraryReportFailure::AdminRestoreOff))
    );
    assert_eq!(versions(&world, &computers).await, before);
}
