//! Game test-support client and player-network fixture setup.

use greybel_test_support::game::Game;
use greyhack_test_support_client::protocol::test_support::{LanNodeKind, NetworkSnapshot};
use greyhack_test_support_client::{TestClient, TestClientOptions};

/// The kind of network [`setup_player`] generates and joins (`ServerMap.TipoRed`); its main router forwards SSH.
pub const DEFAULT_NETWORK_KIND: &str = "Supermercados";

/// Connects a `TestClient` to the game's test-support port.
pub async fn test_client(game: &Game) -> TestClient {
    TestClient::connect(TestClientOptions {
        request_timeout: std::time::Duration::from_secs(120),
        ..TestClientOptions::with_port(game.test_port())
    })
    .await
    .expect("the test-support protocol connects")
}

/// Creates the player fixture, connects it over Wi-Fi to a network generated for the test, stops the game clock and
/// returns its user.
///
/// The clock runs at 14 game seconds per second, so between reading the game's state and running a script in it, NPCs
/// could log in or out, which decides active users, phones and administrators. Stopped, both worlds run the script at
/// the moment the copy was read; a test that needs time to pass moves the clock itself. The next player setup starts
/// it again.
pub async fn setup_player(client: &TestClient) -> String {
    let ready = client
        .setup_player()
        .await
        .expect("the player fixture is created");

    // A network generated for this test alone: within a run each copy of the game serves many tests, and the files,
    // passwords, services and libraries a test's scripts change on a network stay there for the tests after it.
    let network = client
        .find_new_network(DEFAULT_NETWORK_KIND, "ssh")
        .await
        .expect("a new network for the test");

    join_network(client, &network.public_ip).await;
    client
        .freeze_clock()
        .await
        .expect("the game clock is stopped");

    ready.user.to_string()
}

/// The network the player is on, or, when it lacks what `fits` requires, a network generated now whose main router
/// forwards `service`, joined over Wi-Fi. The player's network is generated for each test, so what it has differs.
pub async fn join_network_where(
    client: &TestClient,
    service: &str,
    fits: impl Fn(&NetworkSnapshot) -> bool,
) -> NetworkSnapshot {
    join_network_of_kind_where(
        client,
        &["Supermercados", "Comisaria", "Bancos"],
        service,
        fits,
    )
    .await
}

/// As [`join_network_where`], generating networks of the game's network kinds `kinds` in turn.
pub async fn join_network_of_kind_where(
    client: &TestClient,
    kinds: &[&str],
    service: &str,
    fits: impl Fn(&NetworkSnapshot) -> bool,
) -> NetworkSnapshot {
    join_network_from(client, 0, kinds, service, fits).await
}

/// As [`join_network_where`], but always a network generated now, never the one the player is on. For tests that
/// leave state on the network that later tests in the same copy of the game would read, such as a reported library
/// update, after which the game's first restore of each LAN router fails with a locked database
/// (`Database.RestoreLanRouter` keeps its reader open while `ApplyReportedLibUpdates` writes).
pub async fn join_new_network_where(
    client: &TestClient,
    service: &str,
    fits: impl Fn(&NetworkSnapshot) -> bool,
) -> NetworkSnapshot {
    join_network_from(
        client,
        1,
        &["Supermercados", "Comisaria", "Bancos"],
        service,
        fits,
    )
    .await
}

/// As [`join_new_network_where`], generating networks of the game's network kinds `kinds` in turn.
pub async fn join_new_network_of_kind_where(
    client: &TestClient,
    kinds: &[&str],
    service: &str,
    fits: impl Fn(&NetworkSnapshot) -> bool,
) -> NetworkSnapshot {
    join_network_from(client, 1, kinds, service, fits).await
}

/// Joins the first network from attempt `first` on that has what `fits` requires: attempt 0 is the network the
/// player is on, later ones generate networks of `kinds` in turn.
async fn join_network_from(
    client: &TestClient,
    first: usize,
    kinds: &[&str],
    service: &str,
    fits: impl Fn(&NetworkSnapshot) -> bool,
) -> NetworkSnapshot {
    for attempt in first..first + 32 {
        if attempt > 0 {
            let kind = kinds[attempt % kinds.len()];
            let network = client
                .find_new_network(kind, service)
                .await
                .expect("a network is generated");

            join_network(client, &network.public_ip).await;
        }

        let snapshot = client.network_state().await.expect("the network is read");

        if fits(&snapshot) {
            return snapshot;
        }
    }

    panic!("no generated network has what the test requires");
}

/// Connects the player's computer to the network at `public_ip` over Wi-Fi.
pub async fn join_network(client: &TestClient, public_ip: &str) {
    let connection = client
        .connect_network(public_ip)
        .await
        .expect("the player connects over Wi-Fi");

    assert!(connection.connected, "{}", connection.error);
}

/// The game JSON content of the main router's `kernel_router.so`, as the game has it now.
pub async fn kernel_content(client: &TestClient) -> String {
    let network = client.network_state().await.expect("the network is read");

    network
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Router(router) if node.parent.is_none() => {
                Some(router.kernel_content.to_string())
            }
            _ => None,
        })
        .expect("the main router is in the network")
}

/// The routers and switches between the computer at `local_ip` and the main router of `network`, without either
/// (`NetworkLan.ReversePing` without hubs).
pub fn devices_above(network: &NetworkSnapshot, local_ip: &str) -> Vec<String> {
    let mut index = network
        .nodes
        .iter()
        .position(|node| node.local_ip == local_ip && matches!(node.node, LanNodeKind::Computer(_)))
        .unwrap_or_else(|| panic!("the network has a computer at {local_ip}"));
    let mut devices = Vec::new();

    while let Some(parent) = network.nodes[index].parent {
        index = parent as usize;

        let node = &network.nodes[index];

        if node.parent.is_some() && matches!(node.node, LanNodeKind::Router(_)) {
            devices.push(node.local_ip.to_string());
        }
    }

    devices
}
