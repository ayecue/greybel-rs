use greyhack_world_guilds::{GuildDraft, GuildFailure};

use crate::support::guilds_records::{guild, world_with_companies};

#[tokio::test]
async fn should_not_create_a_guild_twice() {
    // Arrange
    let (mut world, _) = world_with_companies(0).await;
    let draft = |name: &str, account: &str| GuildDraft {
        name: name.to_owned(),
        bank_account: account.to_owned(),
        total_exp: 0,
        investment_perks: 0,
        admin_restore_perks: 0,
        player_member: false,
        player_owner: false,
    };

    guild(&mut world, "Owls", "owls-bank", false).await;

    // Act
    let name = world
        .modules
        .guilds
        .guilds
        .create_guild(&draft("Owls", "other"))
        .await
        .unwrap();
    let account = world
        .modules
        .guilds
        .guilds
        .create_guild(&draft("Hawks", "owls-bank"))
        .await
        .unwrap();

    // Assert
    assert_eq!(name, Err(GuildFailure::NameInUse("Owls".to_owned())));
    assert_eq!(
        account,
        Err(GuildFailure::AccountInUse("owls-bank".to_owned()))
    );
}
