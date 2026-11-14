//! Remote-computer fixtures for scripts that connect, copy files, build and launch through shells.

use greyhack_behaviour::Reward;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{
    ComputerSnapshot, LanNodeKind, LanNodeSnapshot, NetworkSnapshot,
};

use super::game::{devices_above, join_network};
use super::parity::{game_outcomes, snapshot_outcomes};

/// A computer with `library` and an SSH service, with its root credentials from the game snapshot.
pub fn ssh_with_library<'a>(
    snapshot: &'a NetworkSnapshot,
    library: &str,
) -> (&'a ComputerSnapshot, String, u16, String) {
    find_ssh_with_library(snapshot, library)
        .expect("the mirrored network has an SSH computer with the library")
}

/// [`ssh_with_library`], or none when the network has no such computer.
pub fn find_ssh_with_library<'a>(
    snapshot: &'a NetworkSnapshot,
    library: &str,
) -> Option<(&'a ComputerSnapshot, String, u16, String)> {
    snapshot.nodes.iter().find_map(|node| {
        let LanNodeKind::Computer(computer) = &node.node else {
            return None;
        };
        let path = format!("/lib/{library}");
        let port = computer
            .ports
            .iter()
            .find(|port| port.service == "ssh" && !port.closed)?
            .number;
        let root = computer.users.iter().find(|user| user.name == "root")?;

        computer
            .files
            .iter()
            .any(|file| file.path == path)
            .then(|| {
                (
                    computer,
                    node.local_ip.to_string(),
                    port,
                    root.password.to_string(),
                )
            })
    })
}

/// Whether `node` is a visible computer whose gateway is a router, as `NetworkLan.Ping` requires for a connection
/// from another subnet. A computer behind a switch is reachable only from its own subnet.
pub fn reachable_computer(snapshot: &NetworkSnapshot, node: &LanNodeSnapshot) -> bool {
    matches!(node.node, LanNodeKind::Computer(_))
        && !node.hidden
        && snapshot.nodes.iter().any(|gateway| {
            gateway.local_ip == node.gateway
                && matches!(&gateway.node, LanNodeKind::Router(router) if router.kind == "ROUTER")
        })
}

/// [`find_ssh_with_library`] limited to computers the player's computer can log in to by their LAN address. A hidden
/// computer, or one whose gateway is not a router, may be unreachable (`NetworkLan.Ping`), and `connect_service` then
/// answers "Connection refused. Address unreachable". Gives the address, port and root password.
pub fn find_reachable_ssh_with_library(
    snapshot: &NetworkSnapshot,
    library: &str,
) -> Option<(String, u16, String)> {
    snapshot.nodes.iter().find_map(|node| {
        let LanNodeKind::Computer(computer) = &node.node else {
            return None;
        };

        if !reachable_computer(snapshot, node) {
            return None;
        }

        let path = format!("/lib/{library}");
        let port = computer
            .ports
            .iter()
            .find(|port| port.service == "ssh" && !port.closed)?
            .number;
        let root = computer.users.iter().find(|user| user.name == "root")?;

        computer
            .files
            .iter()
            .any(|file| file.path == path)
            .then(|| (node.local_ip.to_string(), port, root.password.to_string()))
    })
}

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

/// The first computer running `service`, with its root password and a non-root account.
pub fn service_account(snapshot: &NetworkSnapshot, service: &str) -> ServiceAccount {
    find_service_account(snapshot, service)
        .unwrap_or_else(|| panic!("the mirrored network has {service} with two accounts"))
}

/// The first computer running `service` with a root and a non-root account, when the network has one.
pub fn find_service_account(snapshot: &NetworkSnapshot, service: &str) -> Option<ServiceAccount> {
    snapshot.nodes.iter().find_map(|node| {
        let LanNodeKind::Computer(computer) = &node.node else {
            return None;
        };
        let port = computer
            .ports
            .iter()
            .find(|port| port.service == service && !port.closed)?
            .number;
        let root = computer.users.iter().find(|user| user.name == "root")?;
        let user = computer.users.iter().find(|user| user.name != "root")?;

        Some(ServiceAccount {
            address: node.local_ip.to_string(),
            port,
            root_password: root.password.to_string(),
            user: user.name.to_string(),
            password: user.password.to_string(),
        })
    })
}

/// The address and port of the first `service` in the mirrored network.
pub fn service(snapshot: &NetworkSnapshot, service: &str) -> (String, u16) {
    find_service(snapshot, service).unwrap_or_else(|| panic!("the mirrored network has {service}"))
}

/// The address and port of the first `service` in the network, when it has one.
pub fn find_service(snapshot: &NetworkSnapshot, service: &str) -> Option<(String, u16)> {
    snapshot.nodes.iter().find_map(|node| {
        let LanNodeKind::Computer(computer) = &node.node else {
            return None;
        };

        computer
            .ports
            .iter()
            .find(|port| port.service == service && !port.closed)
            .map(|port| (node.local_ip.to_string(), port.number))
    })
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

/// A service computer with routers between it and its main router.
pub struct ServiceBehindDevices {
    pub address: String,
    pub port: u16,
    pub root_password: String,
    pub devices: Vec<String>,
}

/// Uses the current network or generates one with the requested service behind another router.
pub async fn service_behind_devices(
    client: &TestClient,
    services: &[&str],
) -> ServiceBehindDevices {
    for attempt in 0..32 {
        if attempt > 0 {
            let kind = ["Supermercados", "Comisaria", "Bancos"][attempt % 3];
            let network = client.find_new_network(kind, services[0]).await.unwrap();

            join_network(client, &network.public_ip).await;
        }

        let snapshot = client.network_state().await.unwrap();

        if let Some(service) = snapshot.nodes.iter().find_map(|node| {
            if node.hidden {
                return None;
            }

            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };
            let port = computer
                .ports
                .iter()
                .find(|port| !port.closed && services.contains(&port.service.as_str()))?;
            let root = computer.users.iter().find(|user| user.name == "root")?;

            // Behind a switch, only its own subnet reaches a computer (`NetworkLan.Ping`); behind a router, the
            // whole LAN does.
            snapshot.nodes.iter().find(|gateway| {
                gateway.local_ip == node.gateway
                    && matches!(&gateway.node, LanNodeKind::Router(router) if router.kind == "ROUTER")
            })?;

            let devices = devices_above(&snapshot, &node.local_ip);

            (!devices.is_empty()).then(|| ServiceBehindDevices {
                address: node.local_ip.to_string(),
                port: port.number,
                root_password: root.password.to_string(),
                devices,
            })
        }) {
            return service;
        }
    }

    panic!("a generated network has {:?} behind a LAN router", services);
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

/// The public port the main router forwards to `service`, and the root password of the computer it leads to.
pub fn forwarded_root(snapshot: &NetworkSnapshot, service: &str) -> (u16, String) {
    let (number, local_ip) = forward(snapshot, service);
    let password = snapshot
        .nodes
        .iter()
        .find_map(|node| {
            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };

            (node.local_ip == local_ip).then_some(())?;

            computer
                .users
                .iter()
                .find(|user| user.name == "root")
                .map(|user| user.password.to_string())
        })
        .expect("the forwarded computer has root");

    (number, password)
}

/// The public port and the LAN address of the main router's forward to `service`.
fn forward(snapshot: &NetworkSnapshot, service: &str) -> (u16, String) {
    snapshot
        .nodes
        .iter()
        .find_map(|node| {
            let LanNodeKind::Router(router) = &node.node else {
                return None;
            };

            node.parent.is_none().then_some(())?;

            router
                .ports
                .iter()
                .find(|port| port.service == service && !port.local_ip.is_empty())
                .map(|port| (port.number, port.local_ip.to_string()))
        })
        .unwrap_or_else(|| panic!("the main router forwards {service}"))
}

/// A rented server network the player joined, with the player's computer running an rshell server whose port its
/// router forwards; the network as the game has it then.
pub async fn rshell_server_network(client: &TestClient) -> NetworkSnapshot {
    let network = client.spawn_network("RentServer").await.unwrap();

    join_network(client, &network.public_ip).await;
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
            LanNodeKind::Computer(computer) if computer.player => Some(node.local_ip.to_string()),
            _ => None,
        })
        .expect("the player's LAN address");

    client.forward_rshell(&local_ip).await.unwrap();

    client.network_state().await.unwrap()
}

/// Whether the simulated copy of `snapshot` opens a `net_use` session to `address:port` for `user`. A computer can pass
/// every check of the snapshot and still be out of the player's reach (a LAN router's subnet, `NetworkLan.Ping`), and
/// a test that compares two failed `net_use` calls compares nothing; the comparison itself still runs in both worlds.
pub fn simulated_net_use(snapshot: &NetworkSnapshot, user: &str, address: &str, port: u16) -> bool {
    let session =
        format!("print typeof(include_lib(\"/lib/metaxploit.so\").net_use(\"{address}\", {port}))");

    snapshot_outcomes(&[&session], user, snapshot)[0]
        .output
        .as_deref()
        .map(str::trim_end)
        == Some("NetSession")
}

/// Whether the simulated copy of `snapshot` logs `user`'s shell in to `address:port` as `account` over SSH. A computer
/// can pass every check of the snapshot and still refuse the login (a firewall on the way, `PlayerUtils.ConnectComputer`),
/// and a test that compares two refused logins compares nothing; the comparison itself still runs in both worlds.
pub fn simulated_login(
    snapshot: &NetworkSnapshot,
    user: &str,
    address: &str,
    port: u16,
    account: &str,
    password: &str,
) -> bool {
    simulated_service_login(snapshot, user, address, port, account, password, "ssh")
}

/// [`simulated_login`] for `service` (`"ssh"` or `"ftp"`), the protocol `ShellIntrinsics.connect_service` is asked for.
pub fn simulated_service_login(
    snapshot: &NetworkSnapshot,
    user: &str,
    address: &str,
    port: u16,
    account: &str,
    password: &str,
    service: &str,
) -> bool {
    let login = format!(
        "print typeof(get_shell.connect_service(\"{address}\", {port}, \"{account}\", {}, \"{service}\"))",
        script_string(password),
    );
    let expected = if service == "ftp" {
        "ftpshell"
    } else {
        "shell"
    };

    snapshot_outcomes(&[&login], user, snapshot)[0]
        .output
        .as_deref()
        .map(str::trim_end)
        == Some(expected)
}
