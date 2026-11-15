//! Guilds and CrownShield: what the game's overflow, flood and vehicle lookup do against a network a guild protects,
//! with the guild, its company and its CrownShield made with the game's own types.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Reward, WorldSeed, license_plate};
use greyhack_test_support_client::GuildSetup;
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{
    join_fixture, police_workstation, setup_player, ssh_network, test_client,
};
use crate::support::metaxploit::{
    computer_library, fixture_reply_weakness, fixture_weakness, overflow_script,
};
use crate::support::parity::assert_succeeded;
use crate::support::parity_guilds::{LEVEL_FIVE, compare_with_guilds};
use crate::support::remote::{fixture_account, program_on_computer};

/// An overflow against a network under CrownShield is interrupted after the game finds its weakness and before it
/// checks anything else, and against the same network without CrownShield it runs: the interrupted attack and the
/// missing weakness are what the game prints in order (`MetaLibIntrinsics.overflow`).
#[tokio::test]
async fn should_match_the_game_when_an_overflow_meets_crownshield() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Gb{suffix}"), &format!("n{suffix}"))
        .await
        .unwrap()
        .bank_number
        .clone();
    // A newly generated company: the game copies keep their worlds between runs, and a company an earlier run's guild
    // bought keeps those shares. Only listed kinds issue shares (`StocksCompany.IsListedOnStockMarket`).
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let ssh_ip = account.address;
    let ssh_port = account.port;

    join_fixture(&client, &network).await;

    let installed = client
        .install_fixture_library(
            &ssh_ip,
            "libssh.so",
            vec![fixture_weakness(Reward::Shell, &[], "guest", "")],
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let library = computer_library(&snapshot, &ssh_ip, "libssh.so");
    let (address, found) = fixture_reply_weakness(&library, &installed.weaknesses[0]);
    let target = Some((ssh_ip.as_str(), ssh_port));
    let scripts = [
        overflow_script("", target, address, found, ""),
        format!(
            "mx = include_lib(\"/lib/metaxploit.so\"); lib = mx.net_use(\"{ssh_ip}\", {ssh_port}).dump_lib; print typeof(lib.overflow(\"0xBAD\", \"missing\"))"
        ),
    ];
    let companies = [snapshot.public_ip.clone()];

    let before = compare_with_guilds(&client, &user, &snapshot, &companies, &scripts).await;

    assert_succeeded(&before);

    assert!(before.iter().all(|outcome| {
        !outcome
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("CrownShield")
    }));

    client
        .setup_guild(GuildSetup {
            name: format!("Owls{suffix}"),
            bank_account: bank,
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            join: true,
            company_ip: snapshot.public_ip.clone(),
            crown: true,
            shield_ip: snapshot.public_ip.clone(),
            admin_restore_perks: 0,
            admin_restore_off: false,
        })
        .await
        .unwrap();

    let after = compare_with_guilds(&client, &user, &snapshot, &companies, &scripts).await;

    assert_succeeded(&after);

    assert!(
        after[0]
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("Attack interrupted: this network is protected by CrownShield."),
        "{:?}",
        after[0]
    );
    assert!(
        !after[1]
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("CrownShield"),
        "{:?}",
        after[1]
    );
    client.shutdown();
}

/// The camera lookup of an administrator's vehicle on a police network under CrownShield
/// (`TrafficNetIntrinsics.locate_vehicle`): refused for the administrator after the plate is found, whatever the
/// vehicle does, and unchanged for the other employees.
#[tokio::test]
async fn should_match_the_game_when_locating_the_vehicle_of_a_protected_administrator() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Gc{suffix}"), &format!("m{suffix}"))
        .await
        .unwrap()
        .bank_number
        .clone();

    // The cameras only work for a program running on a police computer (`TrafficNetIntrinsics.CheckStatus` reads the
    // network of the computer the library is on), and only a police workstation has `libtrafficnet.so`
    // (`Router.PostGenComputer`), so the program is built and launched there through its SSH service.
    let police = police_workstation(&client).await;
    let account = fixture_account(&police, "workstation", "ssh", false);
    let local_ip = account.address;
    let port = account.port;
    let root_password = account.root_password;

    join_fixture(&client, &police).await;

    let plate = client.setup_traffic().await.unwrap().plate.clone();
    let snapshot = client.network_state().await.unwrap();
    let traffic = snapshot.traffic.as_ref().expect("police traffic state");
    let credentials = snapshot
        .npcs
        .iter()
        .find(|npc| npc.user == traffic.credentials_user)
        .expect("the traffic credentials NPC");
    let password = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if node.local_ip == credentials.local_ip => {
                computer.users.iter().find_map(|account| {
                    (account.name == traffic.credentials_user).then(|| account.password.clone())
                })
            }
            _ => None,
        })
        .expect("the traffic account password");
    let admin = snapshot
        .admin
        .as_ref()
        .expect("the network's administrator");
    let admin_plate = snapshot
        .npcs
        .iter()
        .find(|npc| npc.user == admin.user && !npc.license_plate.is_empty())
        .map(|npc| npc.license_plate.clone())
        .expect("the police administrator has a vehicle");
    let unknown = license_plate(
        &WorldSeed::new(snapshot.world_secret.to_vec()),
        &snapshot.public_ip,
        "NoSuchNpc",
    );
    let locate = |plate: &str| {
        program_on_computer(
            &local_ip,
            port,
            &root_password,
            &format!("locate_{}", plate.to_lowercase()),
            &format!(
                "traffic = include_lib(\"/lib/libtrafficnet.so\"); wait(1.1); print traffic.locate_vehicle({plate:?}, {password:?})"
            ),
        )
    };
    let mut scripts = vec![locate(&plate), locate(&unknown.unwrap_or_default())];

    scripts.push(locate(&admin_plate));

    client
        .setup_guild(GuildSetup {
            name: format!("Hawks{suffix}"),
            bank_account: bank,
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            join: true,
            company_ip: snapshot.public_ip.clone(),
            crown: true,
            shield_ip: snapshot.public_ip.clone(),
            admin_restore_perks: 0,
            admin_restore_off: false,
        })
        .await
        .unwrap();

    let companies = [snapshot.public_ip.clone()];
    let outcomes = compare_with_guilds(&client, &user, &snapshot, &companies, &scripts).await;

    assert_succeeded(&outcomes);
    assert!(
        outcomes.iter().all(|outcome| !outcome
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("not registered on any police network")),
        "the program did not run on a police computer: {outcomes:?}"
    );

    assert!(
        outcomes[2]
            .output
            .as_deref()
            .unwrap_or_default()
            .contains("CrownShield provides secure transport"),
        "{:?}",
        outcomes[2]
    );

    client.shutdown();
}
