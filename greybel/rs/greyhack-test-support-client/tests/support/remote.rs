//! Remote-computer fixtures for scripts that connect, copy files, build and launch through shells.

use greyhack_behaviour::Reward;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{
    FixtureNetworkReply, LanNodeKind, NetworkSnapshot,
};

use super::game::{fixture_target, join_network};
use super::parity::game_outcomes;

/// A script that logs in to `address:port` as root, writes `source` to a program called `name` there, builds it and
/// launches it, printing the launch result; what the program prints is part of the output. The program runs on that
/// computer, so `include_lib` there reads its own `/lib`.
pub fn program_on_computer(
    address: &str,
    port: u16,
    root_password: &str,
    name: &str,
    source: &str,
) -> String {
    let source = script_string(source);

    format!(
        "remote = get_shell.connect_service(\"{address}\", {port}, \"root\", \"{root_password}\"); pc = remote.host_computer; pc.touch(\"/root\", \"{name}.src\"); pc.File(\"/root/{name}.src\").set_content({source}); remote.build(\"/root/{name}.src\", \"/root\"); print remote.launch(\"/root/{name}\")"
    )
}

/// Credentials and address of the first computer running `service` in the mirrored network.
pub struct ServiceAccount {
    pub address: String,
    pub port: u16,
    pub root_password: String,
    pub user: String,
    pub password: String,
}

/// Credentials and ports of a named fixture target, using its public address when `public` is set.
pub fn fixture_account(
    network: &FixtureNetworkReply,
    target: &str,
    service: &str,
    public: bool,
) -> ServiceAccount {
    let target = fixture_target(network, target);
    let service = target
        .services
        .iter()
        .find(|candidate| candidate.service == service)
        .unwrap_or_else(|| panic!("the fixture target has {service}"));

    ServiceAccount {
        address: if public {
            network.public_ip.clone()
        } else {
            target.local_ip.clone()
        },
        port: if public {
            service.public_port.expect("the fixture service is public")
        } else {
            service.internal_port
        },
        root_password: target.root_password.clone(),
        user: target.user.clone(),
        password: target.password.clone(),
    }
}

/// The main router's public port forwarding to `service`, when the network exposes it.
pub fn forwarded_service(snapshot: &NetworkSnapshot, service: &str) -> Option<u16> {
    snapshot.nodes.iter().find_map(|node| {
        let LanNodeKind::Router(router) = &node.node else {
            return None;
        };

        node.parent
            .is_none()
            .then(|| {
                router
                    .ports
                    .iter()
                    .find(|port| port.service == service && !port.closed)
                    .map(|port| port.number)
            })
            .flatten()
    })
}

/// `source` as a GreyScript expression of one string: a literal per line, whose quote escape is a doubled quote.
pub fn script_string(source: &str) -> String {
    let lines: Vec<String> = source
        .split('\n')
        .map(|line| format!("\"{}\"", line.replace('"', "\"\"")))
        .collect();

    // A string literal ends at its line (`Lexer.Dequeue`), so the lines are joined with `char(10)`.
    format!("({})", lines.join(" + char(10) + "))
}

/// A rented server network the player joined, with the player's computer running an rshell server whose port its
/// router forwards; the network as the game has it then.
pub async fn rshell_server_network(client: &TestClient, kind: &str) -> NetworkSnapshot {
    let network = client.spawn_network(kind).await.unwrap();

    rshell_server_network_at(client, &network.public_ip).await
}

/// The existing network `public_ip`, joined by the player and prepared for an rshell server.
pub async fn rshell_server_network_at(client: &TestClient, public_ip: &str) -> NetworkSnapshot {
    join_network(client, public_ip).await;
    client
        .install_library(
            "",
            "librshell.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let before = client.network_state().await.unwrap();

    client
        .allow_firewall(&before.nodes[0].local_ip)
        .await
        .unwrap();

    let install = format!(
        "root = get_shell(\"root\", \"test\"); pc = root.host_computer; pc.touch(\"/root\", \"rshell-install.src\"); pc.File(\"/root/rshell-install.src\").set_content({}); root.build(\"/root/rshell-install.src\", \"/root\"); root.launch(\"/root/rshell-install\")",
        script_string("rshell = include_lib(\"/lib/librshell.so\"); rshell.install_service")
    );

    game_outcomes(client, &[install.as_str()]).await;

    let installed = client.network_state().await.unwrap();
    let local_ip = installed
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(node.local_ip.clone()),
            _ => None,
        })
        .expect("the player's LAN address");

    client.forward_rshell(&local_ip).await.unwrap();

    client.network_state().await.unwrap()
}
