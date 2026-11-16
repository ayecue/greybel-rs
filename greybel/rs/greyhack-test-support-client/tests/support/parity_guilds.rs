//! What the guild comparisons share: the game's guilds, company shares and CrownShield networks copied into a copy of
//! its network, and the scripts run in both.

use greybel_test_support::future::block_on;
use greybel_test_support::interpreter::constant_clock_options;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{GuildsReply, NetworkSnapshot};
use greyhack_world::World;
use greyhack_world_guilds::{CompanyLayout, GuildDraft, GuildLayout, GuildsLayout};

use super::intrinsics::{run_many_with_world, snapshot_world};
use super::parity::{Outcome, game_outcomes, outcomes_from_runs, without_users};

/// Experience of a level-5 guild, which may invest in one company.
pub const LEVEL_FIVE: u32 = 275 * 25;

/// The guilds, shareholders and protected networks the game reported, as the world stores them.
pub fn layout(reply: &GuildsReply) -> GuildsLayout {
    GuildsLayout {
        guilds: reply
            .guilds
            .iter()
            .map(|guild| GuildLayout {
                draft: GuildDraft {
                    name: guild.name.clone(),
                    bank_account: guild.bank_account.clone(),
                    total_exp: guild.total_exp,
                    investment_perks: guild.investment_perks,
                    admin_restore_perks: guild.admin_restore_perks,
                    player_member: guild.player_member,
                },
                admin_restore_off: guild
                    .admin_restore_disabled
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                has_crown: guild.has_crown,
                crown_shield_public_ip: guild.crown_shield_public_ip.clone(),
            })
            .collect(),
        companies: reply
            .companies
            .iter()
            .map(|company| CompanyLayout {
                public_ip: company.public_ip.clone(),
                seed: company.seed,
                holders: company
                    .holders
                    .iter()
                    .map(|holder| (holder.bank_account.clone(), holder.shares))
                    .collect(),
            })
            .collect(),
        shields: reply
            .shields
            .iter()
            .map(|shield| (shield.public_ip.clone(), shield.guild_name.clone()))
            .collect(),
    }
}

/// A copy of the game's player network with the game's guilds.
pub fn mirrored_world(user: &str, snapshot: &NetworkSnapshot, reply: &GuildsReply) -> World {
    let world = snapshot_world(user, snapshot);

    block_on(world.modules.guilds.guilds.mirror_guilds(&layout(reply)))
        .expect("the guilds are mirrored");

    world
}

/// The outcome of each script run in order in one copy of the game's guilds and network.
pub fn mirrored_outcomes(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
    reply: &GuildsReply,
) -> Vec<Outcome> {
    let results = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                run_many_with_world(
                    scripts,
                    user,
                    mirrored_world(user, snapshot, reply),
                    constant_clock_options,
                )
            })
            .join()
            .expect("the intrinsics run")
    });

    outcomes_from_runs(scripts, results)
}

/// Runs `scripts` in the game and in a copy of its network and guilds, asserts both give the same, and returns the
/// game's outcomes. `company_ips` are the companies whose shareholders the copy takes.
pub async fn compare_with_guilds(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    company_ips: &[String],
    scripts: &[String],
) -> Vec<Outcome> {
    compare_with_guilds_labeled(client, user, snapshot, company_ips, "guilds", scripts).await
}

/// [`compare_with_guilds`] with the fixture state included in a failed assertion.
pub async fn compare_with_guilds_labeled(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    company_ips: &[String],
    label: &str,
    scripts: &[String],
) -> Vec<Outcome> {
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let reply = client
        .query_guilds(company_ips)
        .await
        .expect("the guilds are read");
    // A shell reward from a normal user's weakness names a user the game picks at random (`ScriptsUtil.GetShellUser`).
    let expected = without_users(mirrored_outcomes(&scripts, user, snapshot, &reply));
    let actual = without_users(game_outcomes(client, &scripts).await);

    assert_eq!(actual, expected, "{label}");

    actual
}
