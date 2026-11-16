//! NPCs: network administrators, rented servers' administrators, lateness and forced logins.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Reward, SECONDS_PER_DAY, day_of_year, is_weekend, parse_date};
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{join_fixture, join_network, setup_player, ssh_network, test_client};
use crate::support::metaxploit::{
    computer_library, fixture_reply_weakness, fixture_weakness, overflow_script, router_library,
};
use crate::support::parity::{compare_in, compare_in_any_user, compare_labeled};
use crate::support::remote::fixture_account;
use crate::support::steps::{Step, WorldChange, compare_stepped};

/// A network's super administrator forces every attack on its computers to the guest user until they are fired
/// (`NpcInfo.IsSuperAdmin`, `NetworkLan.FireNpc`).
#[tokio::test]
async fn should_match_intrinsics_for_a_super_admin_until_fired() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let address = network.public_ip.clone();
    let account = fixture_account(&network, "ssh", "ssh", false);

    join_fixture(&client, &network).await;
    client.promote_admin(&address).await.unwrap();

    let installed = client
        .install_fixture_library(
            &account.address,
            "libssh.so",
            [Reward::Shell, Reward::Computer, Reward::RandomFolder]
                .into_iter()
                .map(|reward| fixture_weakness(reward, &[], "guest", ""))
                .collect(),
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let library = computer_library(&snapshot, &account.address, "libssh.so");
    let scripts: Vec<String> = installed
        .weaknesses
        .iter()
        .map(|fixture| {
            let (zone, weakness) = fixture_reply_weakness(&library, fixture);

            overflow_script(
                "",
                Some((&account.address, account.port)),
                zone,
                weakness,
                "",
            )
        })
        .collect();

    assert_eq!(scripts.len(), 3);

    let intercepted = compare_in(&client, &user, &snapshot, &scripts).await;

    assert!(intercepted.iter().any(|outcome| {
        outcome
            .output
            .as_deref()
            .is_some_and(|output| output.contains("attack intercepted by administrator"))
    }));

    let (zone, weakness) = fixture_reply_weakness(&library, &installed.weaknesses[0]);
    let guest_homes = format!(
        "shell = mx.net_use(\"{}\", {}).dump_lib.overflow(\"{zone}\", \"{}\"); pc = shell.host_computer; for home in pc.File(\"/home\").get_folders; print [home.path, home.owner, home.group, home.permissions, pc.touch(home.path, \"x.src\")]; end for",
        account.address, account.port, weakness.unsec_value
    );

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "super-admin-guest-home-permissions",
        &[guest_homes],
    )
    .await;

    let admin = snapshot.admin.as_ref().unwrap().user.clone();

    client.fire_npc(&address, &admin).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let fired = compare_in_any_user(&client, &user, &snapshot, &scripts).await;

    assert!(fired.iter().all(|outcome| {
        !outcome
            .output
            .as_deref()
            .is_some_and(|output| output.contains("intercepted"))
    }));
    client.shutdown();
}

/// The administrator a rented server's network hired forces every attack on it to the guest user while alive
/// (`NetworkLan.IsSuperAdminRental`).
#[tokio::test]
async fn should_match_intrinsics_for_a_rented_servers_admin() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = client.spawn_network("RentServer").await.unwrap();
    let address = network.public_ip.clone();

    join_network(&client, &address).await;
    client.hire_rental_admin(&address).await.unwrap();

    let initial = client.network_state().await.unwrap();

    assert!(initial.rental_admin);

    let main = initial.nodes[0].local_ip.clone();
    let installed = client
        .install_fixture_library(
            &main,
            "kernel_router.so",
            [Reward::Shell, Reward::Computer, Reward::RandomFolder]
                .into_iter()
                .map(|reward| fixture_weakness(reward, &[], "guest", ""))
                .collect(),
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let kernel = router_library(&snapshot, &main);
    let scripts: Vec<String> = installed
        .weaknesses
        .iter()
        .map(|fixture| {
            let (zone, weakness) = fixture_reply_weakness(&kernel, fixture);

            overflow_script("", Some((&main, 0)), zone, weakness, &main)
        })
        .collect();

    assert_eq!(scripts.len(), 3);

    let outcomes = compare_in(&client, &user, &snapshot, &scripts).await;

    assert!(outcomes.iter().any(|outcome| {
        outcome
            .output
            .as_deref()
            .is_some_and(|output| output.contains("attack intercepted by administrator"))
    }));
    client.shutdown();
}

/// A deleted phone alarm makes the administrator late: their work login and network restore wait until the wake-up.
/// Repeated lateness counts strikes for an ordinary NPC too, and the third fires them and refuses their SSH account.
#[tokio::test]
async fn should_match_intrinsics_for_npc_lateness() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let address = network.public_ip.clone();
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let root_password = account.root_password;

    join_network(&client, &address).await;
    client.freeze_clock().await.unwrap();

    let initial = client.network_state().await.unwrap();
    let admin = initial.admin.as_ref().expect("the network administrator");
    let admin_user = admin.user.clone();
    let target_user = account.user;

    assert_ne!(target_user, admin_user);

    for (npc_user, disable, strikes) in [
        (admin_user.clone(), true, 1),
        (target_user.clone(), false, 3),
    ] {
        for strike in 0..strikes {
            let before = client.network_state().await.unwrap();
            let now = parse_date(&before.current_date);
            let mut aligned = (now / SECONDS_PER_DAY + 1) * SECONDS_PER_DAY + 6 * 3_600;

            while is_weekend(aligned) || is_weekend(aligned + SECONDS_PER_DAY) {
                aligned += SECONDS_PER_DAY;
            }

            client.advance_clock(aligned - now).await.unwrap();
            if disable && strike == 0 {
                client.disable_network(&address, true).await.unwrap();
            }

            client
                .set_phone_alarm(&address, &npc_user, 0, 0, true)
                .await
                .unwrap();

            let snapshot = client.network_state().await.unwrap();
            let npc = snapshot
                .npcs
                .iter()
                .find(|npc| npc.user == npc_user)
                .expect("the NPC's lateness");
            let now = parse_date(&snapshot.current_date);
            let midnight = now / SECONDS_PER_DAY * SECONDS_PER_DAY;
            let mut work = midnight + npc.schedule_work1_start;

            while work <= now || is_weekend(work) {
                work += SECONDS_PER_DAY;
            }

            let mut alarm_day = midnight;

            if npc.day_alarm != day_of_year(now) {
                alarm_day += SECONDS_PER_DAY;
            }

            let wake = alarm_day + npc.wake_up_time.expect("the deleted alarm wakes late");
            let script = format!(
                "print typeof(get_shell.connect_service(\"{address}\", {port}, \"root\", \"{root_password}\"))\nuser_input(\"\")\nprint typeof(get_shell.connect_service(\"{address}\", {port}, \"root\", \"{root_password}\"))\nuser_input(\"\")\nprint typeof(get_shell.connect_service(\"{address}\", {port}, \"root\", \"{root_password}\"))"
            );

            compare_stepped(
                &client,
                &user,
                &snapshot,
                &script,
                &[
                    Step::new(vec![WorldChange::AdvanceClock(work - now)], ""),
                    Step::new(vec![WorldChange::AdvanceClock(wake - work)], ""),
                ],
            )
            .await;

            let after = client.network_state().await.unwrap();
            let after = after
                .npcs
                .iter()
                .find(|npc| npc.user == npc_user)
                .expect("the NPC after their alarm");

            assert_eq!(after.late_strikes, strike + 1);
        }
    }

    let snapshot = client.network_state().await.unwrap();
    let npc = snapshot
        .npcs
        .iter()
        .find(|npc| npc.user == target_user)
        .expect("the fired NPC");

    assert_eq!(npc.late_strikes, 3);
    assert!(npc.fired);

    let (lan_ip, port, password) = snapshot
        .nodes
        .iter()
        .find_map(|node| {
            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };
            let account = computer
                .users
                .iter()
                .find(|user| user.name == target_user)?;
            let port = computer
                .ports
                .iter()
                .find(|port| port.service == "ssh" && !port.closed)?;

            Some((node.local_ip.clone(), port.number, account.password.clone()))
        })
        .expect("the fired NPC's SSH account");
    let script = format!(
        "print get_shell.connect_service(\"{lan_ip}\", {port}, \"{target_user}\", \"{password}\")"
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// A network's administrator forced to log in, as a social-engineering mail does, counts as an active root user for an
/// hour also off shift, then logs out (`Persona.ForceLogin`, `Persona.IsLogged`, `Computer.IsRootActiveUser`).
#[tokio::test]
async fn should_match_intrinsics_for_a_forced_admin_login() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let address = network.public_ip.clone();
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;

    let before = join_fixture(&client, &network).await;
    let admin = before.admin.as_ref().expect("the network's administrator");
    let schedule = before
        .npcs
        .iter()
        .find(|npc| npc.user == admin.user)
        .expect("the administrator's schedule");
    // The middle of their time off: the two work blocks of four hours take up to a day from the first one's start.
    let off_start = schedule.schedule_work2_start + 4 * 3_600;
    let off_end = schedule.schedule_work1_start + 24 * 3_600;
    let middle = u64::midpoint(off_start, off_end) % 86_400;
    let now = greyhack_behaviour::parse_date(&before.current_date) % 86_400;

    client
        .advance_clock((middle + 86_400 - now) % 86_400)
        .await
        .unwrap();
    client.force_admin_login(&address).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    // The fixture's forwarded SSH computer is reachable from outside.
    let ip = snapshot.public_ip.clone();
    let script = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
active = function()
  return mx.net_use("{ip}", {port}).is_root_active_user
end function
print "forced " + active
user_input("")
print "half an hour later " + active
user_input("")
print "an hour later " + active"#
    );
    let outcome = compare_stepped(
        &client,
        &user,
        &snapshot,
        &script,
        &[
            Step::new(vec![WorldChange::AdvanceClock(1_800)], ""),
            Step::new(vec![WorldChange::AdvanceClock(1_801)], ""),
        ],
    )
    .await;

    assert_eq!(
        outcome.output.as_deref(),
        Some("forced 1\nhalf an hour later 1\nan hour later 0"),
        "{outcome:?}"
    );
    client.shutdown();
}
