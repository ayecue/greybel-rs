use greyhack_world::World;
use greyhack_world_core::ComputerId;
use greyhack_world_guilds::GuildDraft;

use super::library_updates::{SSH_PATH, network_of, version_of};

/// More shares than any company issues, so the holder owns all of it.
const ALL_SHARES: u32 = 2_000_000;

/// Experience of a level-8 guild, which may invest in two companies (`GuildLevelRules.GetMaxInvestedCompanies`).
pub const LEVEL_EIGHT: u32 = 275 * 64;

/// Experience of a level-4 guild, which may not invest yet.
pub const LEVEL_FOUR: u32 = 275 * 16;

/// Creates guild `name` holding company shares under `account`.
pub async fn guild(world: &mut World, name: &str, account: &str, total_exp: u32, perks: u32) {
    world
        .modules
        .guilds
        .guilds
        .create_guild(&GuildDraft {
            name: name.to_owned(),
            bank_account: account.to_owned(),
            total_exp,
            investment_perks: 5,
            admin_restore_perks: perks,
            player_member: false,
            player_owner: false,
        })
        .await
        .unwrap()
        .unwrap();
}

/// A network of two computers whose libssh.so is older than the world's newest, owned by `account`, and its address.
pub async fn company(world: &mut World, account: &str) -> (String, Vec<ComputerId>) {
    let (public_ip, computers) = network_of(world, &["1.0.0", "1.0.1"]).await;

    assert!(
        world
            .modules
            .guilds
            .guilds
            .set_company_shares(&public_ip, account, ALL_SHARES)
            .await
            .unwrap()
    );

    (public_ip, computers)
}

/// The libssh.so versions of `computers`.
pub async fn versions(world: &World, computers: &[ComputerId]) -> Vec<String> {
    let mut found = Vec::new();

    for computer in computers {
        found.push(version_of(world, *computer, SSH_PATH).await);
    }

    found
}
