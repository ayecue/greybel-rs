//! Smart appliances and traffic: their rewards, settings, cameras and accidents.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Requirement, Reward, WorldSeed, license_plate};
use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};
use greyhack_world_libraries::Weakness;

use crate::support::game::{
    join_network, join_new_network_of_kind_where, setup_player, test_client,
};
use crate::support::metaxploit::{computer_library, overflow_script, player_libraries, weakness};
use crate::support::parity::{assert_succeeded, compare_in, compare_in_on_waiting_clock};
use crate::support::remote::{
    find_reachable_ssh_with_library, reachable_computer, script_string, simulated_login,
    simulated_net_use,
};

/// The rewards only the smart-appliance and traffic libraries have: on the player's computer of the fixture network,
/// where neither target qualifies; on a police network for the traffic lights; on a fridge of another network for the
/// override settings.
#[tokio::test]
async fn should_match_intrinsics_for_appliance_and_traffic_rewards() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let rewards = [
        ("libsmartappliance.so", Reward::OverrideSettings),
        ("libtrafficnet.so", Reward::TrafficLightControl),
    ];

    for (name, reward) in rewards {
        client
            .install_library("", name, u8::try_from(reward.code()).unwrap())
            .await
            .unwrap();
    }

    let local_scripts = |snapshot: &NetworkSnapshot| -> Vec<String> {
        rewards
            .iter()
            .map(|(name, reward)| {
                let path = format!("/lib/{name}");
                let library = player_libraries(snapshot)
                    .into_iter()
                    .find(|(file, _)| *file == path)
                    .map(|(_, library)| library)
                    .expect("the installed library");
                let (address, weakness) = weakness(&library, |weakness| weakness.reward == *reward)
                    .expect("the library has the reward");

                overflow_script(&path, None, address, weakness, "")
            })
            .collect()
    };
    let snapshot = client.network_state().await.unwrap();

    compare_in(&client, &user, &snapshot, &local_scripts(&snapshot)).await;

    let police = client.find_network("Comisaria", "").await.unwrap();

    join_network(&client, &police.public_ip).await;

    let snapshot = client.network_state().await.unwrap();

    compare_in(&client, &user, &snapshot, &local_scripts(&snapshot)).await;

    let appliances = client.find_network("FastFood", "FRIDGE").await.unwrap();

    join_network(&client, &appliances.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let (fridge, port) = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.kind == "FRIDGE" => computer
                .ports
                .iter()
                .find(|port| port.service == "smartappliance")
                .map(|port| (node.local_ip.to_string(), port.number)),
            _ => None,
        })
        .expect("the network has a fridge with its service");

    client
        .install_library(
            &fridge,
            "libsmartappliance.so",
            u8::try_from(Reward::OverrideSettings.code()).unwrap(),
        )
        .await
        .unwrap();
    client.lock_appliance(&fridge).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let library = computer_library(&snapshot, &fridge, "libsmartappliance.so");
    let (address, weakness) = weakness(&library, |weakness| {
        weakness.reward == Reward::OverrideSettings && weakness.requirements.is_empty()
    })
    .expect("the fridge's library has the override");
    // Twice in one run: enabling the override, then finding it enabled.
    let script = overflow_script("", Some((&fridge, port)), address, weakness, "");

    compare_in(&client, &user, &snapshot, &[format!("{script}; {script}")]).await;
    client.shutdown();
}

/// A traffic-light overflow on a police computer, through SSH, SCP and a source program launched there.
#[tokio::test]
async fn should_match_intrinsics_for_traffic_lights_over_a_remote_shell() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    // Only a police workstation has `libtrafficnet.so` (`Router.PostGenComputer`), and not every police network has one
    // with SSH.
    let police = join_new_network_of_kind_where(&client, &["Comisaria"], "ssh", |snapshot| {
        find_reachable_ssh_with_library(snapshot, "libtrafficnet.so").is_some_and(
            |(address, port, password)| {
                simulated_login(snapshot, &user, &address, port, "root", &password)
            },
        )
    })
    .await;
    let (local_ip, _, _) = find_reachable_ssh_with_library(&police, "libtrafficnet.so")
        .expect("a reachable police workstation with SSH");

    client
        .install_library(
            &local_ip,
            "libtrafficnet.so",
            u8::try_from(Reward::TrafficLightControl.code()).unwrap(),
        )
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let (local_ip, port, password) = find_reachable_ssh_with_library(&snapshot, "libtrafficnet.so")
        .expect("the police workstation is still reachable");
    let library = computer_library(&snapshot, &local_ip, "libtrafficnet.so");
    let (address, weakness) = weakness(&library, |weakness| {
        weakness.reward == Reward::TrafficLightControl
            && weakness.requirements.iter().all(|requirement| {
                matches!(
                    requirement,
                    Requirement::NumberUsersRegister | Requirement::PathExist
                )
            })
    })
    .expect("the police library has a traffic weakness with satisfiable requirements");
    let source = format!(
        "mx = include_lib(\"/root/metaxploit.so\"); lib = mx.load(\"/lib/libtrafficnet.so\"); result = lib.overflow(\"{address}\", \"{}\", \"\"); print typeof(result)",
        weakness.unsec_value
    );
    let source = script_string(&source);
    let script = format!(
        "remote = get_shell.connect_service(\"{local_ip}\", {port}, \"root\", \"{password}\"); print typeof(remote); print remote.scp(\"/lib/metaxploit.so\", \"/root\", get_shell, 1); pc = remote.host_computer; pc.touch(\"/root\", \"traffic.src\"); print pc.File(\"/root/traffic.src\").set_content({source}); print remote.build(\"/root/traffic.src\", \"/root\"); print remote.launch(\"/root/traffic\")"
    );
    let outcomes = compare_in(&client, &user, &snapshot, &[script]).await;

    assert_succeeded(&outcomes);
    assert!(
        outcomes[0]
            .output
            .as_deref()
            .is_some_and(|output| output.contains("Traffic lights override ready")),
        "the program on the police computer did not run the overflow: {outcomes:?}"
    );
    client.shutdown();
}

/// The appliance methods on a fridge, before and after the override reward. `include_lib` only gives the appliance
/// object to a program running on the appliance itself (`PlayerIntrinsics.include_lib` reads the host computer's
/// `/lib`), and the override reward only returns 1 (`MetaLibIntrinsics.overflow`), so each script takes a shell on the
/// fridge through its service library, builds a program there and launches it.
#[tokio::test]
async fn should_match_intrinsics_for_smart_appliance_settings() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let free = |weakness: &Weakness, reward: Reward| {
        weakness.reward == reward && weakness.requirements.is_empty()
    };
    let mut selected = None;
    let mut skipped = Vec::new();

    'networks: for _ in 0..12 {
        let network = client.find_network("FastFood", "FRIDGE").await.unwrap();

        join_network(&client, &network.public_ip).await;

        let before = client.network_state().await.unwrap();
        let Some((fridge, port)) = before.nodes.iter().find_map(|node| match &node.node {
            LanNodeKind::Computer(computer)
                if computer.kind == "FRIDGE" && reachable_computer(&before, node) =>
            {
                computer
                    .ports
                    .iter()
                    .find(|port| port.service == "smartappliance")
                    .map(|port| (node.local_ip.to_string(), port.number))
            }
            _ => None,
        }) else {
            skipped.push(format!("{}: no reachable fridge", network.public_ip));
            continue;
        };

        // The fixture installs the game's generated version whose weaknesses for both rewards have the fewest
        // requirements in total. Which versions exist changes as the game releases new ones (`VersionsControl`).
        client
            .install_library_with(
                &fridge,
                "libsmartappliance.so",
                &[
                    u8::try_from(Reward::OverrideSettings.code()).unwrap(),
                    u8::try_from(Reward::Shell.code()).unwrap(),
                ],
            )
            .await
            .unwrap();

        let snapshot = client.network_state().await.unwrap();
        let library = computer_library(&snapshot, &fridge, "libsmartappliance.so");
        let unlock = weakness(&library, |weakness| {
            free(weakness, Reward::OverrideSettings)
        })
        .map(|(zone, weakness)| (zone.to_owned(), weakness.clone()));
        let shell = weakness(&library, |weakness| free(weakness, Reward::Shell))
            .map(|(zone, weakness)| (zone.to_owned(), weakness.clone()));

        let (Some(unlock), Some(shell)) = (unlock, shell) else {
            // Every fridge gets the same version, the one whose two weaknesses need the fewest requirements, so
            // another network can't help when that version still needs some: the game releases a new version
            // instead (`VersionsControl.CheckUpdatePrograms` draws new weaknesses for it).
            client
                .release_library_version("libsmartappliance.so")
                .await
                .unwrap();
            skipped.push(format!(
                "{}: {} lacks free weaknesses",
                network.public_ip, library.version
            ));
            continue;
        };

        if simulated_net_use(&snapshot, &user, &fridge, port) {
            selected = Some((snapshot, fridge, port, unlock, shell));
            break 'networks;
        }
        skipped.push(format!(
            "{}: net_use to {fridge} refused in the copy",
            network.public_ip
        ));
    }

    let Some((
        snapshot,
        fridge,
        port,
        (unlock_zone, unlock_weakness),
        (shell_zone, shell_weakness),
    )) = selected
    else {
        panic!(
            "a fridge library with an override and a shell weakness; networks tried: {skipped:#?}"
        );
    };
    // The shell's user decides where it may write: root and guest have fixed homes, and for a normal user the game
    // picks one at random (`ScriptsUtil.GetShellUser`), so the script takes the first home it can write in.
    let folder = match shell_weakness.user.as_str() {
        "root" => "folder = \"/root\"".to_owned(),
        "guest" => "folder = \"/home/guest\"".to_owned(),
        _ => "folder = \"\"
for home in pc.File(\"/home\").get_folders
if pc.touch(home.path, \"app.src\") == 1 then
folder = home.path
break
end if
end for"
            .to_owned(),
    };
    let program = script_string(
        "app = include_lib(\"/lib/libsmartappliance.so\")
print typeof(app)
print app.model
print app.set_alarm(1)
print app.override_settings(900, 200)
print app.set_alarm(0)
print app.override_settings(900, 200)",
    );
    let on_fridge = format!(
        "shell = mx.net_use(\"{fridge}\", {port}).dump_lib.overflow(\"{shell_zone}\", \"{}\")
print typeof(shell)
pc = shell.host_computer
{folder}
pc.touch(folder, \"app.src\")
print pc.File(folder + \"/app.src\").set_content({program})
print shell.build(folder + \"/app.src\", folder)
print shell.launch(folder + \"/app\")",
        shell_weakness.unsec_value
    );
    let locked = format!("mx = include_lib(\"/lib/metaxploit.so\")\n{on_fridge}");
    let unlock = overflow_script(
        "",
        Some((&fridge, port)),
        &unlock_zone,
        &unlock_weakness,
        "",
    );
    let unlocked = format!("{unlock}\n{on_fridge}");
    let outcomes = compare_in(&client, &user, &snapshot, &[locked, unlocked]).await;

    assert_succeeded(&outcomes);
    assert!(
        outcomes.iter().all(|outcome| outcome
            .output
            .as_deref()
            .is_some_and(|output| !output.contains("Error"))),
        "the program on the fridge failed: {outcomes:?}"
    );
    client.shutdown();
}

/// Police traffic cameras: registration, throttling, credential/plate failures, a commuting driver, the real accident,
/// its cooldown, the used override after cooldown expiry, and a disabled police network.
#[tokio::test]
async fn should_match_intrinsics_for_traffic_cameras_and_accidents() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    // The player's computer has no traffic library of its own (the game makes `libtrafficnet.so` only on police
    // workstations, `Router.PostGenComputer`), so `include_lib` would return null and every script would fail alike.
    client
        .install_library(
            "",
            "libtrafficnet.so",
            u8::try_from(Reward::TrafficLightControl.code()).unwrap(),
        )
        .await
        .unwrap();

    let initial = client.network_state().await.unwrap();
    let non_police = r#"traffic = include_lib("/lib/libtrafficnet.so")
print traffic.get_credentials_info
print traffic.camera_link_system
print traffic.locate_vehicle("bad", "bad")"#
        .to_owned();

    assert_succeeded(&compare_in(&client, &user, &initial, &[non_police]).await);

    // `TrafficNetIntrinsics.CheckStatus` reads the network of the computer the library is on, and only a police
    // workstation has `libtrafficnet.so` (`Router.PostGenComputer`): the police steps run as a program there, which the
    // player's script builds through the workstation's SSH service.
    join_new_network_of_kind_where(&client, &["Comisaria"], "ssh", |snapshot| {
        find_reachable_ssh_with_library(snapshot, "libtrafficnet.so").is_some()
    })
    .await;

    let plate = client.setup_traffic().await.unwrap().plate.to_string();
    let snapshot = client.network_state().await.unwrap();
    let (address, port, root_password) =
        find_reachable_ssh_with_library(&snapshot, "libtrafficnet.so")
            .expect("a reachable police workstation with the library");
    let on_police = |name: &str, source: &str| {
        let source = script_string(source);

        // Print the login result and skip the remote program on failure: a failed comparison then shows why
        // `connect_service` refused the login instead of only the later `host_computer` lookup error.
        format!(
            "remote = get_shell.connect_service(\"{address}\", {port}, \"root\", \"{root_password}\")
print remote
if typeof(remote) == \"shell\" then
pc = remote.host_computer
pc.touch(\"/root\", \"{name}.src\")
pc.File(\"/root/{name}.src\").set_content({source})
remote.build(\"/root/{name}.src\", \"/root\")
print remote.launch(\"/root/{name}\")
else
print \"remote login failed\"
end if"
        )
    };
    let traffic = snapshot.traffic.as_ref().expect("police traffic state");
    let credentials_npc = snapshot
        .npcs
        .iter()
        .find(|npc| npc.user == traffic.credentials_user)
        .expect("the traffic credentials NPC");
    let password = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if node.local_ip == credentials_npc.local_ip => {
                computer.users.iter().find_map(|user| {
                    (user.name == traffic.credentials_user).then(|| user.password.to_string())
                })
            }
            _ => None,
        })
        .expect("the traffic account password");
    let out_of_sight = snapshot
        .npcs
        .iter()
        .find(|npc| !npc.commuting && !npc.license_plate.is_empty() && npc.license_plate != plate)
        .map(|npc| npc.license_plate.to_string());
    let unknown = license_plate(
        &WorldSeed::new(snapshot.world_secret.to_vec()),
        &snapshot.public_ip,
        "NoSuchNpc",
    )
    .expect("a plate for a police network address");
    let mut source = format!(
        r#"traffic = include_lib("/lib/libtrafficnet.so")
print traffic.get_credentials_info
print traffic.camera_link_system
print traffic.camera_link_system
print traffic.locate_vehicle("bad", {plate:?})
print traffic.locate_vehicle("bad", {password:?})
print traffic.locate_vehicle({unknown:?}, {password:?})"#
    );

    if let Some(out_of_sight) = &out_of_sight {
        source.push_str(&format!(
            "\nprint traffic.locate_vehicle({out_of_sight:?}, {password:?})"
        ));
    }

    source.push_str(&format!(
        "\nwait(1.1)\nprint traffic.locate_vehicle({plate:?}, {password:?})"
    ));

    // `camera_link_system` and `locate_vehicle` share a one-second throttle (`TrafficNetIntrinsics`), which the
    // script's `wait` must pass.
    let registered =
        compare_in_on_waiting_clock(&client, &user, &snapshot, &[on_police("cameras", &source)])
            .await;

    assert_succeeded(&registered);

    let mut police_outcomes = registered;

    let driver = snapshot
        .npcs
        .iter()
        .find(|npc| npc.license_plate == plate)
        .expect("the located driver");

    client
        .traffic_accident(&driver.user, &snapshot.public_ip, &snapshot.public_ip)
        .await
        .unwrap();

    let after = client.network_state().await.unwrap();
    let during_cooldown = format!(
        "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.camera_link_system; print traffic.locate_vehicle({plate:?}, {password:?})"
    );
    let cooldown = compare_in(
        &client,
        &user,
        &after,
        &[on_police("cooldown", &during_cooldown)],
    )
    .await;

    assert_succeeded(&cooldown);
    police_outcomes.extend(cooldown);
    client.reset_traffic_cooldown().await.unwrap();

    let expired = client.network_state().await.unwrap();
    let cameras_off = format!(
        "traffic = include_lib(\"/lib/libtrafficnet.so\"); wait(1.1); print traffic.camera_link_system; print traffic.locate_vehicle({plate:?}, {password:?})"
    );
    let off = compare_in_on_waiting_clock(
        &client,
        &user,
        &expired,
        &[on_police("cameras_off", &cameras_off)],
    )
    .await;

    assert_succeeded(&off);
    police_outcomes.extend(off);
    assert!(
        police_outcomes.iter().all(|outcome| {
            let output = outcome.output.as_deref().unwrap_or_default();

            !output.contains("not registered on any police network")
                && !output.contains("remote login failed")
        }),
        "a program did not run on a police computer: {police_outcomes:?}"
    );

    client
        .disable_network(&snapshot.public_ip, true)
        .await
        .unwrap();

    let disabled = client.network_state().await.unwrap();
    let offline = format!(
        "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.get_credentials_info; print traffic.camera_link_system; print traffic.locate_vehicle({plate:?}, {password:?})"
    );

    assert_succeeded(&compare_in(&client, &user, &disabled, &[offline]).await);
    client.shutdown();
}
