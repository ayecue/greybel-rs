//! Traffic credential, driver, and throttle faults.

use greybel_test_support::game::Game;

use crate::support::game::{join_fixture, police_workstation, setup_player, test_client};
use crate::support::parity::{compare_in_on_waiting_clock, compare_labeled};
use crate::support::remote::{fixture_account, script_string};

fn traffic_password(
    snapshot: &greyhack_test_support_client::protocol::test_support::NetworkSnapshot,
) -> String {
    let traffic = snapshot.traffic.as_ref().unwrap();
    let npc = snapshot
        .npcs
        .iter()
        .find(|npc| npc.user == traffic.credentials_user)
        .unwrap();
    snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            greyhack_test_support_client::protocol::test_support::LanNodeKind::Computer(
                computer,
            ) if node.local_ip == npc.local_ip => computer
                .users
                .iter()
                .find(|user| user.name == traffic.credentials_user)
                .map(|user| user.password.clone()),
            _ => None,
        })
        .unwrap()
}

fn on_police(
    snapshot: &greyhack_test_support_client::protocol::test_support::NetworkSnapshot,
    name: &str,
    source: &str,
) -> String {
    let account = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            greyhack_test_support_client::protocol::test_support::LanNodeKind::Computer(
                computer,
            ) if computer
                .ports
                .iter()
                .any(|port| port.service == "ssh" && port.number == 10022) =>
            {
                Some((
                    node.local_ip.clone(),
                    computer
                        .users
                        .iter()
                        .find(|user| user.name == "root")
                        .unwrap()
                        .password
                        .clone(),
                ))
            }
            _ => None,
        })
        .unwrap();
    let source = script_string(source);
    format!(
        "remote = get_shell.connect_service(\"{}\", 10022, \"root\", \"{}\"); print typeof(remote); pc = remote.host_computer; pc.touch(\"/root\", \"{name}.src\"); pc.File(\"/root/{name}.src\").set_content({source}); print remote.build(\"/root/{name}.src\", \"/root\"); print remote.launch(\"/root/{name}\")",
        account.0, account.1
    )
}

/// A missing credentials workstation and a driver who is no longer alive are distinct from a bad plate.
#[tokio::test]
async fn should_match_intrinsics_for_traffic_credential_and_driver_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    for fixture in ["missing_traffic_workstation", "dead_driver"] {
        let network = police_workstation(&client).await;
        let account = fixture_account(&network, "workstation", "ssh", false);
        let address = account.address;
        let port = account.port;
        let password = account.root_password;
        join_fixture(&client, &network).await;
        let plate = client.setup_traffic().await.unwrap().plate.clone();
        let before = client.network_state().await.unwrap();
        let credentials = traffic_password(&before);
        if fixture == "missing_traffic_workstation" {
            client
                .configure_computer(&address, "missing_traffic_workstation")
                .await
                .unwrap();
        } else {
            let driver = before
                .npcs
                .iter()
                .find(|npc| npc.license_plate == plate)
                .unwrap();
            client
                .fire_npc(&before.public_ip, &driver.user)
                .await
                .unwrap();
        }
        let snapshot = client.network_state().await.unwrap();
        let source = if fixture == "missing_traffic_workstation" {
            "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.camera_link_system"
                .to_owned()
        } else {
            format!(
                "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.locate_vehicle({plate:?}, {credentials:?})"
            )
        };
        let source = script_string(&source);
        let script = format!(
            "remote = get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\"); print typeof(remote); pc = remote.host_computer; pc.touch(\"/root\", \"{fixture}.src\"); pc.File(\"/root/{fixture}.src\").set_content({source}); print remote.build(\"/root/{fixture}.src\", \"/root\"); print remote.launch(\"/root/{fixture}\")"
        );

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    client.shutdown();
}

/// Locate has its own one-second throttle, even without a preceding camera call.
#[tokio::test]
async fn should_match_intrinsics_for_traffic_locate_throttle() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = police_workstation(&client).await;
    join_fixture(&client, &network).await;
    let plate = client.setup_traffic().await.unwrap().plate.clone();
    let snapshot = client.network_state().await.unwrap();
    let credentials = traffic_password(&snapshot);
    let source = format!(
        "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.locate_vehicle({plate:?}, {credentials:?}); print traffic.locate_vehicle({plate:?}, {credentials:?})"
    );
    let script = on_police(&snapshot, "locate_throttle", &source);

    compare_in_on_waiting_clock(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// The traffic fixture identifies the actual dead driver, not merely a vehicle's other occupants.
#[tokio::test]
async fn should_match_intrinsics_for_a_dead_traffic_driver() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = police_workstation(&client).await;

    join_fixture(&client, &network).await;
    let plate = client.setup_traffic().await.unwrap().plate;
    let snapshot = client.network_state().await.unwrap();
    let credentials = traffic_password(&snapshot);
    let locate = on_police(
        &snapshot,
        "locate_for_dead_driver",
        &format!(
            "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.locate_vehicle({plate:?}, {credentials:?})"
        ),
    );
    compare_in_on_waiting_clock(&client, &user, &snapshot, &[locate]).await;
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
    let dead_driver = after
        .traffic
        .as_ref()
        .unwrap()
        .accident_driver
        .as_deref()
        .unwrap();
    let source = format!(
        "traffic = include_lib(\"/lib/libtrafficnet.so\"); print traffic.driver_is_alive({dead_driver:?}, {credentials:?})"
    );
    let script = on_police(&after, "dead_driver", &source);

    compare_labeled(&client, &user, &after, "dead-driver", &[script]).await;
    client.shutdown();
}

/// Broken and wrong-type appliance maps have the same null behavior as the game.
#[tokio::test]
async fn should_match_intrinsics_for_broken_appliances() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = [
        "print get_router(\"203.0.113.1\").appliances[\"fridge\"]".to_owned(),
        "print get_router(\"203.0.113.1\").appliances[1]".to_owned(),
    ];

    compare_labeled(&client, &user, &snapshot, "broken-appliances", &scripts).await;
    client.shutdown();
}
