//! Game test-support client and player-network fixture setup.

use greybel_test_support::game::Game;
use greyhack_test_support_client::protocol::test_support::{
    FixtureNetworkReply, FixtureService, FixtureTarget, FixtureTargetReply, LanNodeKind,
    NetworkSnapshot,
};
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
    let network = ssh_network(client).await;

    join_network(client, &network.public_ip).await;
    client
        .freeze_clock()
        .await
        .expect("the game clock is stopped");

    ready.user.clone()
}

/// Creates a fresh supermarket with a named, publicly forwarded SSH computer.
pub async fn ssh_network(client: &TestClient) -> FixtureNetworkReply {
    ssh_network_of_kind(client, DEFAULT_NETWORK_KIND).await
}

/// Creates a fresh network of `kind` with a named, publicly forwarded SSH computer.
pub async fn ssh_network_of_kind(client: &TestClient, kind: &str) -> FixtureNetworkReply {
    service_network(client, kind, "ssh", 10_022, 10_022).await
}

/// Creates a fresh network with a named public service computer.
pub async fn service_network(
    client: &TestClient,
    kind: &str,
    service: &str,
    internal_port: u16,
    public_port: u16,
) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            kind,
            "flat",
            false,
            vec![FixtureTarget {
                name: service.to_owned(),
                device: "COMPUTER".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: service.to_owned(),
                    internal_port,
                    public_port: Some(public_port),
                    oldest_library: false,
                }],
            }],
        )
        .await
        .expect("the service fixture is created")
}

/// Creates a fresh public SSH target whose library is the world's first generated libssh version.
pub async fn old_ssh_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            DEFAULT_NETWORK_KIND,
            "flat",
            false,
            vec![FixtureTarget {
                name: "ssh".to_owned(),
                device: "COMPUTER".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: "ssh".to_owned(),
                    internal_port: 10_022,
                    public_port: Some(10_022),
                    oldest_library: true,
                }],
            }],
        )
        .await
        .expect("the old SSH fixture is created")
}

/// Creates a fresh supermarket whose main router has the world's first generated kernel.
pub async fn router_update_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(DEFAULT_NETWORK_KIND, "flat", true, Vec::new())
        .await
        .expect("the router update fixture is created")
}

/// Creates one public computer running both SSH and FTP.
pub async fn dual_service_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            DEFAULT_NETWORK_KIND,
            "flat",
            false,
            vec![FixtureTarget {
                name: "dual".to_owned(),
                device: "COMPUTER".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![
                    FixtureService {
                        service: "ssh".to_owned(),
                        internal_port: 10_022,
                        public_port: Some(10_022),
                        oldest_library: false,
                    },
                    FixtureService {
                        service: "ftp".to_owned(),
                        internal_port: 10_021,
                        public_port: Some(10_021),
                        oldest_library: false,
                    },
                ],
            }],
        )
        .await
        .expect("the dual-service fixture is created")
}

/// Creates a fresh MailServices network with a publicly forwarded SMTP computer.
pub async fn smtp_network(client: &TestClient) -> FixtureNetworkReply {
    service_network(client, "MailServices", "smtp", 10_025, 10_025).await
}

/// Creates a fresh supermarket with an SMTP computer on public port 25.
pub async fn public_smtp_network(client: &TestClient) -> FixtureNetworkReply {
    service_network(client, DEFAULT_NETWORK_KIND, "smtp", 25, 25).await
}

/// Creates a fresh supermarket with a publicly forwarded FTP computer.
pub async fn ftp_network(client: &TestClient) -> FixtureNetworkReply {
    service_network(client, DEFAULT_NETWORK_KIND, "ftp", 10_021, 10_021).await
}

/// Creates a fresh supermarket with public SSH and a child router, optionally with a switch too.
pub async fn router_ssh_network(client: &TestClient, switch: bool) -> FixtureNetworkReply {
    router_ssh_network_of_kind(client, DEFAULT_NETWORK_KIND, switch).await
}

/// [`router_ssh_network`] on the selected network kind.
pub async fn router_ssh_network_of_kind(
    client: &TestClient,
    kind: &str,
    switch: bool,
) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            kind,
            if switch { "router_switch" } else { "router" },
            false,
            vec![FixtureTarget {
                name: "ssh".to_owned(),
                device: "COMPUTER".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: "ssh".to_owned(),
                    internal_port: 10_022,
                    public_port: Some(10_022),
                    oldest_library: false,
                }],
            }],
        )
        .await
        .expect("the router fixture is created")
}

/// Creates a fresh supermarket with an SSH computer behind its guaranteed Wi-Fi child router.
pub async fn routed_ssh_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            DEFAULT_NETWORK_KIND,
            "router",
            false,
            vec![FixtureTarget {
                name: "ssh".to_owned(),
                device: "COMPUTER".to_owned(),
                location: "lan_router".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: "ssh".to_owned(),
                    internal_port: 10_022,
                    public_port: None,
                    oldest_library: false,
                }],
            }],
        )
        .await
        .expect("the routed SSH fixture is created")
}

/// Creates a fresh supermarket with a smartphone running SSH.
pub async fn smartphone_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            DEFAULT_NETWORK_KIND,
            "flat",
            false,
            vec![FixtureTarget {
                name: "phone".to_owned(),
                device: "SMARTPHONE".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: "ssh".to_owned(),
                    internal_port: 10_022,
                    public_port: Some(10_022),
                    oldest_library: false,
                }],
            }],
        )
        .await
        .expect("the smartphone fixture is created")
}

/// Creates a fresh FastFood network with a reachable smart-appliance fridge.
pub async fn fridge_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            "FastFood",
            "flat",
            false,
            vec![FixtureTarget {
                name: "fridge".to_owned(),
                device: "FRIDGE".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: "smartappliance".to_owned(),
                    internal_port: 1221,
                    public_port: None,
                    oldest_library: false,
                }],
            }],
        )
        .await
        .expect("the fridge fixture is created")
}

/// Creates a fridge on an adminless RentServer network so an overflow is not intercepted.
pub async fn adminless_fridge_network(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            "RentServer",
            "flat",
            false,
            vec![FixtureTarget {
                name: "fridge".to_owned(),
                device: "FRIDGE".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![FixtureService {
                    service: "smartappliance".to_owned(),
                    internal_port: 1221,
                    public_port: None,
                    oldest_library: false,
                }],
            }],
        )
        .await
        .expect("the adminless fridge fixture is created")
}

/// Creates a fresh network of `kind` without extra targets; its generated content remains intact.
pub async fn empty_network(client: &TestClient, kind: &str) -> FixtureNetworkReply {
    client
        .create_fixture_network(kind, "flat", false, Vec::new())
        .await
        .expect("the empty fixture is created")
}

/// Creates a fresh Comisaria with its criminals service and a public SSH workstation.
pub async fn police_workstation(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            "Comisaria",
            "flat",
            false,
            vec![
                FixtureTarget {
                    name: "criminals".to_owned(),
                    device: "COMPUTER".to_owned(),
                    location: "main".to_owned(),
                    root_password: String::new(),
                    library_state: String::new(),
                    services: vec![FixtureService {
                        service: "criminals".to_owned(),
                        internal_port: 154,
                        public_port: None,
                        oldest_library: false,
                    }],
                },
                FixtureTarget {
                    name: "workstation".to_owned(),
                    device: "COMPUTER".to_owned(),
                    location: "main".to_owned(),
                    root_password: String::new(),
                    library_state: String::new(),
                    services: vec![FixtureService {
                        service: "ssh".to_owned(),
                        internal_port: 10_022,
                        public_port: Some(10_022),
                        oldest_library: false,
                    }],
                },
            ],
        )
        .await
        .expect("the police workstation fixture is created")
}

/// Creates a fresh HackShop with HTTP and its generated repository on one target.
pub async fn hackshop_repository(client: &TestClient) -> FixtureNetworkReply {
    client
        .create_fixture_network(
            "HackShop",
            "flat",
            false,
            vec![FixtureTarget {
                name: "repository".to_owned(),
                device: "COMPUTER".to_owned(),
                location: "main".to_owned(),
                root_password: String::new(),
                library_state: String::new(),
                services: vec![
                    FixtureService {
                        service: "http".to_owned(),
                        internal_port: 80,
                        public_port: None,
                        oldest_library: false,
                    },
                    FixtureService {
                        service: "repository".to_owned(),
                        internal_port: 6312,
                        public_port: None,
                        oldest_library: false,
                    },
                ],
            }],
        )
        .await
        .expect("the HackShop repository fixture is created")
}

/// Creates a fresh enabled MailServices network with SMTP and `npc_computers` extra NPC workstations.
pub async fn mail_network(client: &TestClient, npc_computers: usize) -> FixtureNetworkReply {
    let mut targets = vec![FixtureTarget {
        name: "smtp".to_owned(),
        device: "COMPUTER".to_owned(),
        location: "main".to_owned(),
        root_password: String::new(),
        library_state: String::new(),
        services: vec![FixtureService {
            service: "smtp".to_owned(),
            internal_port: 10_025,
            public_port: Some(10_025),
            oldest_library: false,
        }],
    }];

    targets.extend((0..npc_computers).map(|index| FixtureTarget {
        name: format!("npc{index}"),
        device: "COMPUTER".to_owned(),
        location: "main".to_owned(),
        root_password: String::new(),
        library_state: String::new(),
        services: Vec::new(),
    }));

    client
        .create_fixture_network("MailServices", "flat", false, targets)
        .await
        .expect("the mail fixture is created")
}

/// Returns a named target from a fixture reply.
pub fn fixture_target<'a>(network: &'a FixtureNetworkReply, name: &str) -> &'a FixtureTargetReply {
    network
        .targets
        .iter()
        .find(|target| target.name == name)
        .unwrap_or_else(|| panic!("the fixture has target {name}"))
}

/// Joins a created fixture and reads its complete server-side state.
pub async fn join_fixture(client: &TestClient, network: &FixtureNetworkReply) -> NetworkSnapshot {
    join_network(client, &network.public_ip).await;

    client.network_state().await.expect("the fixture is read")
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
                Some(router.kernel_content.clone())
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
            devices.push(node.local_ip.clone());
        }
    }

    devices
}
