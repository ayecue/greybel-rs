//! What the guild comparisons share: the game's guilds, company shares and CrownShield networks copied into a copy of
//! its network, and the scripts run in both.

use greybel_interpreter::RuntimeErrorKind;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack::run_on_host_at;
use greybel_test_support::interpreter::constant_clock_options;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{GuildsReply, NetworkSnapshot};
use greyhack_world::{CompanyLayout, GuildDraft, GuildLayout, GuildsLayout, World};

use super::intrinsics::{PROGRAM_NAME, snapshot_world};
use super::parity::{Outcome, game_outcomes, without_ack_counts, without_users};

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
                    name: guild.name.to_string(),
                    bank_account: guild.bank_account.to_string(),
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
                crown_shield_public_ip: guild.crown_shield_public_ip.to_string(),
            })
            .collect(),
        companies: reply
            .companies
            .iter()
            .map(|company| CompanyLayout {
                public_ip: company.public_ip.to_string(),
                seed: company.seed,
                holders: company
                    .holders
                    .iter()
                    .map(|holder| (holder.bank_account.to_string(), holder.shares))
                    .collect(),
            })
            .collect(),
        shields: reply
            .shields
            .iter()
            .map(|shield| (shield.public_ip.to_string(), shield.guild_name.to_string()))
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

/// The outcome of each script run in its own copy of the game's guilds and network.
pub fn mirrored_outcomes(
    scripts: &[&str],
    user: &str,
    snapshot: &NetworkSnapshot,
    reply: &GuildsReply,
) -> Vec<Outcome> {
    scripts
        .iter()
        .map(|script| {
            // On a thread of its own: the run blocks on its own executor, which can't run inside the test's runtime.
            let (output, error) = std::thread::scope(|scope| {
                scope
                    .spawn(|| {
                        let world = mirrored_world(user, snapshot, reply);
                        let path = format!("/home/{user}/{PROGRAM_NAME}");
                        let run = run_on_host_at(
                            script,
                            world,
                            user,
                            true,
                            &path,
                            constant_clock_options(),
                        );

                        (run.terminal, run.error)
                    })
                    .join()
                    .expect("the intrinsics run")
            });

            Outcome {
                script: (*script).to_owned(),
                failed: error.is_some(),
                output: error.is_none().then(|| without_ack_counts(&output)),
                error: error.filter(|error| *error != RuntimeErrorKind::GameCrash.to_string()),
            }
        })
        .collect()
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
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let reply = client
        .query_guilds(company_ips)
        .await
        .expect("the guilds are read");
    // A shell reward from a normal user's weakness names a user the game picks at random (`ScriptsUtil.GetShellUser`).
    let expected = without_users(mirrored_outcomes(&scripts, user, snapshot, &reply));
    let actual = without_users(game_outcomes(client, &scripts).await);

    assert_eq!(actual, expected);

    actual
}
