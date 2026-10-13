//! NPCs: network administrators, rented servers' administrators, lateness and forced logins.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Reward, SECONDS_PER_DAY, day_of_year, is_weekend, parse_date};
use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};

use crate::support::game::{join_network, setup_player, test_client};
use crate::support::metaxploit::{computer_library, overflow_script, router_library, weakness};
use crate::support::parity::{compare_in, compare_in_any_user};
use crate::support::remote::{forwarded_root, service};
use crate::support::steps::{Step, WorldChange, compare_stepped};

/// A network's super administrator forces every attack on its computers to the guest user until they are fired
/// (`NpcInfo.IsSuperAdmin`, `NetworkLan.FireNpc`).
#[tokio::test]
async fn should_match_intrinsics_for_a_super_admin_until_fired() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();
    let address = network.public_ip.to_string();

    join_network(&client, &address).await;
    client.promote_admin(&address).await.unwrap();

    let rewards = [Reward::Shell, Reward::Computer, Reward::RandomFolder];
    let overflows = |snapshot: &NetworkSnapshot| -> Vec<String> {
        let (ip, port) = service(snapshot, "ssh");
        let library = computer_library(snapshot, &ip, "libssh.so");

        rewards
            .into_iter()
            .filter_map(|reward| {
                weakness(&library, |weakness| {
                    weakness.reward == reward && weakness.requirements.is_empty()
                })
                .map(|(zone, weakness)| overflow_script("", Some((&ip, port)), zone, weakness, ""))
            })
            .collect()
    };
    let mut snapshot = client.network_state().await.unwrap();

    // The network's own libssh.so may have no weakness of these rewards without requirements.
    if overflows(&snapshot).is_empty() {
        let (ip, _) = service(&snapshot, "ssh");

        client
            .install_library(
                &ip,
                "libssh.so",
                u8::try_from(Reward::Shell.code()).unwrap(),
            )
            .await
            .unwrap();
        snapshot = client.network_state().await.unwrap();
    }

    let scripts = overflows(&snapshot);

    assert!(!scripts.is_empty());

    let intercepted = compare_in(&client, &user, &snapshot, &scripts).await;

    assert!(intercepted.iter().any(|outcome| {
        outcome
            .output
            .as_deref()
            .is_some_and(|output| output.contains("attack intercepted by administrator"))
    }));

    let admin = snapshot.admin.as_ref().unwrap().user.to_string();

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
    let address = network.public_ip.to_string();

    join_network(&client, &address).await;
    client.hire_rental_admin(&address).await.unwrap();

    let snapshot = client.network_state().await.unwrap();

    assert!(snapshot.rental_admin);

    let main = snapshot.nodes[0].local_ip.to_string();
    let kernel = router_library(&snapshot, &main);
    let scripts: Vec<String> = [Reward::Shell, Reward::Computer, Reward::RandomFolder]
        .into_iter()
        .filter_map(|reward| {
            weakness(&kernel, |weakness| {
                weakness.reward == reward && weakness.requirements.is_empty()
            })
            .map(|(zone, weakness)| overflow_script("", Some((&main, 0)), zone, weakness, &main))
        })
        .collect();

    assert!(!scripts.is_empty());

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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();
    let address = network.public_ip.to_string();

    join_network(&client, &address).await;
    client.freeze_clock().await.unwrap();

    let initial = client.network_state().await.unwrap();
    let admin = initial.admin.as_ref().expect("the network administrator");
    let admin_user = admin.user.to_string();
    let (port, root_password) = forwarded_root(&initial, "ssh");
    let target_user = initial
        .nodes
        .iter()
        .find_map(|node| {
            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };

            if !computer
                .ports
                .iter()
                .any(|port| port.service == "ssh" && !port.closed)
            {
                return None;
            }

            computer
                .users
                .iter()
                .find(|user| user.name != "root" && user.name != admin_user)
                .map(|user| user.name.to_string())
        })
        .expect("an ordinary NPC with an SSH account");

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

            Some((
                node.local_ip.to_string(),
                port.number,
                account.password.to_string(),
            ))
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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();
    let address = network.public_ip.to_string();

    join_network(&client, &address).await;

    let before = client.network_state().await.unwrap();
    let admin = before.admin.as_ref().expect("the network's administrator");
    let schedule = before
        .npcs
        .iter()
        .find(|npc| npc.user == admin.user)
        .expect("the administrator's schedule");
    // The middle of their time off: the two work blocks of four hours take up to a day from the first one's start.
    let off_start = schedule.schedule_work2_start + 4 * 3_600;
    let off_end = schedule.schedule_work1_start + 24 * 3_600;
    let middle = ((off_start + off_end) / 2) % 86_400;
    let now = greyhack_behaviour::parse_date(&before.current_date) % 86_400;

    client
        .advance_clock((middle + 86_400 - now) % 86_400)
        .await
        .unwrap();
    client.force_admin_login(&address).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    // The forwarded SSH computer is reachable from outside; a LAN computer may not be from the player's subnet.
    let (port, _) = forwarded_root(&snapshot, "ssh");
    let ip = snapshot.public_ip.to_string();
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
