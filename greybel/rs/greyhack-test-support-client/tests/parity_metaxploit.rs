//! metaxploit: libraries, net sessions, overflows and their requirements, reverse shells and sniffers.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Requirement, Reward};
use greyhack_test_support_client::protocol::test_support::LanNodeKind;
use greyhack_world::Weakness;

use crate::support::game::{join_network, join_network_where, setup_player, test_client};
use crate::support::metaxploit::{
    computer_library, overflow_script, player_libraries, router_libraries, router_library, weakness,
};
use crate::support::mirror::library;
use crate::support::parity::{
    assert_succeeded, compare_in, compare_in_any_user, compare_in_real_time, compare_pair_in,
    game_outcomes, snapshot_outcomes, without_users,
};
use crate::support::remote::{
    find_service_account, find_ssh_with_library, rshell_server_network, script_string,
    service_account, ssh_with_library,
};

/// The libraries of the mirrored network, including the game-installed metaxploit, scanned and reached as in the game.
#[tokio::test]
async fn should_match_intrinsics_for_metaxploit_library_discovery() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let player = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            greyhack_test_support_client::protocol::test_support::LanNodeKind::Computer(
                computer,
            ) if computer.player => Some(computer),
            _ => None,
        })
        .unwrap();

    assert_eq!(player.active_net_card, "WIFI");

    let mut scripts = vec![
        "mx = include_lib(\"/lib/metaxploit.so\"); print typeof(mx); print mx.load(\"/lib/missing.so\")"
            .to_owned(),
    ];

    for file in player.files.iter().filter(|file| {
        matches!(file.path.as_str(), "/lib/metaxploit.so" | "/lib/crypto.so")
            && library(&file.content).is_some()
    }) {
        let library = library(&file.content).unwrap();
        let zone = &library.zones[0];
        let script = format!(
            "mx = include_lib(\"/lib/metaxploit.so\"); lib = mx.load(\"{}\"); print [typeof(lib), lib.lib_name, lib.version]; print mx.scan(lib); print mx.scan_address(lib, \"{}\"); print mx.scan_address(lib, \"0xBAD\")",
            file.path, zone.address
        );

        scripts.push(script);
    }

    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = snapshot_outcomes(&scripts, &user, &snapshot);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// `net_use` on routers, LAN services and public forwarded ports of the mirrored network.
#[tokio::test]
async fn should_match_intrinsics_for_metaxploit_net_use() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| match &snapshot.nodes[0].node {
        LanNodeKind::Router(router) => router
            .ports
            .iter()
            .any(|port| port.service == "ssh" && !port.local_ip.is_empty()),
        _ => false,
    })
    .await;
    let mut targets = snapshot
        .nodes
        .iter()
        .filter_map(|node| match &node.node {
            LanNodeKind::Router(router) => library(&router.kernel_content)
                .is_some()
                .then(|| (node.local_ip.to_string(), 0)),
            LanNodeKind::Computer(computer) => computer
                .ports
                .iter()
                .filter(|port| !port.closed)
                .find(|port| {
                    let path = format!("/lib/lib{}.so", port.service);

                    computer
                        .files
                        .iter()
                        .any(|file| file.path == path && library(&file.content).is_some())
                })
                .map(|port| (node.local_ip.to_string(), port.number)),
            LanNodeKind::Hub => None,
        })
        .collect::<Vec<_>>();
    let forwarded = match &snapshot.nodes[0].node {
        LanNodeKind::Router(router) => {
            router
                .ports
                .iter()
                .find(|port| port.service == "ssh" && !port.local_ip.is_empty())
                .expect("the main router forwards SSH")
                .number
        }
        _ => unreachable!(),
    };

    targets.push((snapshot.public_ip.to_string(), forwarded));

    let targets = targets
        .iter()
        .map(|(ip, port)| format!("[\"{ip}\", {port}]"))
        .collect::<Vec<_>>()
        .join(", ");
    let computer_ip = snapshot
        .nodes
        .iter()
        .find(|node| matches!(node.node, LanNodeKind::Computer(_)))
        .expect("a computer in the network")
        .local_ip
        .to_string();
    let scripts = [
        "pc = get_shell.host_computer; print [pc.active_net_card, pc.public_ip, pc.local_ip, pc.network_gateway, pc.is_network_active]".to_owned(),
        format!("mx = include_lib(\"/lib/metaxploit.so\")\nfor target in [{targets}]\n  session = mx.net_use(target[0], target[1]); print [target, typeof(session)]\n  if typeof(session) == \"NetSession\" then\n    lib = session.dump_lib; print [lib.lib_name, lib.version]\n  end if\nend for"),
        format!("mx = include_lib(\"/lib/metaxploit.so\"); print mx.net_use(\"bad\", 22); print mx.net_use(\"{computer_ip}\", 9999); print mx.net_use(\"10.0.0.1\", 22); print mx.net_use(\"{}\", 9999)", snapshot.public_ip),
    ];
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = snapshot_outcomes(&scripts, &user, &snapshot);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// Local overflow rewards, requirement failures, password payloads and missing vulnerabilities.
#[tokio::test]
async fn should_match_intrinsics_for_local_overflows() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let (_, library) = player_libraries(&snapshot)
        .into_iter()
        .find(|(path, _)| path == "/lib/metaxploit.so")
        .unwrap();
    let mut scripts = vec![
        "mx = include_lib(\"/lib/metaxploit.so\"); lib = mx.load(\"/lib/metaxploit.so\"); print lib.overflow(\"0xBAD\", \"missing\"); print lib.overflow(1, \"missing\")"
            .to_owned(),
    ];

    for reward in [
        Reward::Shell,
        Reward::Computer,
        Reward::RandomFolder,
        Reward::ChangePass,
    ] {
        if let Some((address, weakness)) = weakness(&library, |weakness| {
            weakness.reward == reward && weakness.requirements.is_empty()
        }) {
            scripts.push(overflow_script(
                "/lib/metaxploit.so",
                None,
                address,
                weakness,
                "newpass123",
            ));
        }
    }

    let mut covered = Vec::new();

    for requirement in Requirement::ALL {
        if let Some((address, weakness)) = weakness(&library, |weakness| {
            weakness.requirements.contains(requirement)
        }) {
            covered.push(*requirement);
            scripts.push(overflow_script(
                "/lib/metaxploit.so",
                None,
                address,
                weakness,
                "newpass123",
            ));
        }
    }

    for requirement in [
        Requirement::Library,
        Requirement::NumberUsersRegister,
        Requirement::AnyActiveUser,
        Requirement::RootActiveUser,
        Requirement::GuestActiveUser,
        Requirement::PathExist,
    ] {
        assert!(covered.contains(&requirement), "missing {requirement:?}");
    }

    assert!(
        scripts.len() >= 6,
        "the fixture covers rewards and requirements"
    );

    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = snapshot_outcomes(&scripts, &user, &snapshot);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// Service-library overflow requirements reached through `net_use`.
#[tokio::test]
async fn should_match_intrinsics_for_remote_overflows() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_ssh_with_library(snapshot, "libssh.so").is_some()
    })
    .await;
    let (_, ssh_ip, ssh_port, _) = ssh_with_library(&snapshot, "libssh.so");
    let library = computer_library(&snapshot, &ssh_ip, "libssh.so");
    let mut scripts = Vec::new();

    for reward in [Reward::Shell, Reward::Computer, Reward::RandomFolder] {
        if let Some((address, weakness)) = weakness(&library, |weakness| {
            weakness.reward == reward
                && weakness.requirements.is_empty()
                && weakness.user != "normal_user"
        }) {
            scripts.push(overflow_script(
                "",
                Some((&ssh_ip, ssh_port)),
                address,
                weakness,
                "",
            ));
        }
    }

    let mut covered = Vec::new();

    for requirement in Requirement::ALL {
        if let Some((address, weakness)) = weakness(&library, |weakness| {
            weakness.requirements.contains(requirement)
        }) {
            covered.push(*requirement);
            scripts.push(overflow_script(
                "",
                Some((&ssh_ip, ssh_port)),
                address,
                weakness,
                "",
            ));
        }
    }

    assert!(covered.contains(&Requirement::PortForward));
    assert!(!scripts.is_empty());

    compare_in_any_user(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// Router shell and computer rewards, subnet and gateway requirements, and firewall disabling on a switch.
#[tokio::test]
async fn should_match_intrinsics_for_router_overflows() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        let kernels = router_libraries(snapshot);
        let main_ip = snapshot.nodes[0].local_ip.as_str();
        let any = |choose: &dyn Fn(&Weakness) -> bool| {
            kernels
                .iter()
                .any(|(_, library)| weakness(library, choose).is_some())
        };

        find_ssh_with_library(snapshot, "libssh.so").is_some()
            && kernels.iter().any(|(ip, _)| ip.as_str() != main_ip)
            && any(&|weakness| {
                weakness.reward == Reward::FirewallDisable && weakness.requirements.is_empty()
            })
            && any(&|weakness| {
                matches!(weakness.reward, Reward::Shell | Reward::Computer)
                    && weakness.requirements.is_empty()
                    && weakness.user != "normal_user"
            })
    })
    .await;
    let kernels = router_libraries(&snapshot);
    let (_, ssh_ip, _, _) = ssh_with_library(&snapshot, "libssh.so");
    let main_ip = snapshot.nodes[0].local_ip.as_str();
    let mut scripts = Vec::new();
    let mut firewall = false;

    for reward in [Reward::Shell, Reward::Computer] {
        for (ip, library) in &kernels {
            if let Some((address, weakness)) = weakness(library, |weakness| {
                weakness.reward == reward
                    && weakness.requirements.is_empty()
                    && weakness.user != "normal_user"
            }) {
                scripts.push(overflow_script(
                    "",
                    Some((ip, 0)),
                    address,
                    weakness,
                    &ssh_ip,
                ));
                break;
            }
        }
    }

    for requirement in [Requirement::LocalNetwork, Requirement::ConnGateway] {
        for (ip, library) in &kernels {
            if let Some((address, weakness)) = weakness(library, |weakness| {
                weakness.requirements.contains(&requirement)
            }) {
                scripts.push(overflow_script(
                    "",
                    Some((ip, 0)),
                    address,
                    weakness,
                    &ssh_ip,
                ));
            }
        }
    }

    for (ip, library) in &kernels {
        if let Some((address, weakness)) = weakness(library, |weakness| {
            weakness.reward == Reward::FirewallDisable && weakness.requirements.is_empty()
        }) {
            firewall = true;
            scripts.push(format!(
                "{}; print get_router(\"{ip}\").firewall_rules",
                overflow_script("", Some((ip, 0)), address, weakness, "",)
            ));
            break;
        }
    }

    assert!(kernels.iter().any(|(ip, _)| ip.as_str() != main_ip));

    assert!(firewall, "the fixture covers firewall disabling");
    assert!(scripts.len() >= 2, "the fixture covers router rewards");

    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = without_users(snapshot_outcomes(&scripts, &user, &snapshot));

    let actual = without_users(game_outcomes(&client, &scripts).await);

    assert_eq!(actual, expected);
    client.shutdown();
}

/// Router requirements selected from kernel versions the game generated, restoring each original kernel afterwards.
#[tokio::test]
async fn should_match_intrinsics_for_router_requirements() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    let initial = join_network_where(&client, "ssh", |snapshot| {
        let main = snapshot.nodes[0].local_ip.to_string();

        find_ssh_with_library(snapshot, "libssh.so").is_some()
            && router_libraries(snapshot)
                .into_iter()
                .any(|(ip, _)| ip != main)
    })
    .await;
    let main = initial.nodes[0].local_ip.to_string();
    let lan = router_libraries(&initial)
        .into_iter()
        .map(|(ip, _)| ip)
        .find(|ip| ip != &main)
        .expect("a LAN router with a kernel library");
    let (_, ssh_ip, _, _) = ssh_with_library(&initial, "libssh.so");

    // The main router is in the player's subnet, the LAN router isn't.
    for (router, requirement) in [
        (main.as_str(), Requirement::ConnGateway),
        (main.as_str(), Requirement::LocalNetwork),
        (lan.as_str(), Requirement::LocalNetwork),
    ] {
        let original = client
            .set_router_kernel_requirement(router, u8::try_from(requirement.code()).unwrap())
            .await
            .unwrap();
        let snapshot = client.network_state().await.unwrap();
        let kernel = router_library(&snapshot, router);
        let (address, weakness) = weakness(&kernel, |weakness| {
            weakness.requirements.contains(&requirement)
        })
        .unwrap_or_else(|| panic!("missing {requirement:?} in {kernel:?}"));
        let script = overflow_script("", Some((router, 0)), address, weakness, &ssh_ip);
        // A computer reward of a `normal_user` weakness names a user the game draws at random.
        let expected = without_users(snapshot_outcomes(&[&script], &user, &snapshot));

        let actual = without_users(game_outcomes(&client, &[&script]).await);

        client
            .write_router_kernel(router, &original.original)
            .await
            .unwrap();
        assert_eq!(actual, expected, "{requirement:?}");
    }

    client.shutdown();
}

/// Reverse-shell validation, the missing service and forward failures, then a client a launched program connects, which
/// the launching program's rshell server lists.
#[tokio::test]
async fn should_match_intrinsics_for_reverse_shells() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = client.spawn_network("RentServer").await.unwrap();

    join_network(&client, &network.public_ip).await;
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

    let before = client.network_state().await.unwrap();
    let public_ip = before.public_ip.to_string();
    let failures = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
print mx.rshell_client("bad", 1222, "client")
print mx.rshell_client("240.0.0.1", 1222, "client")
print mx.rshell_client("{public_ip}", 9999, "client")
print mx.rshell_client("{public_ip}", 1222, "bad name")
print mx.rshell_client("{public_ip}", 1222, "1234567890123456789012345")
print mx.rshell_client("{public_ip}", 1222, "kernel_task")
print mx.rshell_server"#
    );

    compare_in(&client, &user, &before, &[failures]).await;

    let install = format!(
        "root = get_shell(\"root\", \"test\"); pc = root.host_computer; pc.touch(\"/root\", \"rshell-install.src\"); pc.File(\"/root/rshell-install.src\").set_content({}); print root.build(\"/root/rshell-install.src\", \"/root\"); print root.launch(\"/root/rshell-install\")",
        script_string("rshell = include_lib(\"/lib/librshell.so\"); print rshell.install_service")
    );

    compare_in(&client, &user, &before, &[install]).await;

    let installed = client.network_state().await.unwrap();
    let no_forward = r#"print include_lib("/lib/metaxploit.so").rshell_server"#.to_owned();

    compare_in(&client, &user, &installed, &[no_forward]).await;

    let local_ip = installed
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(node.local_ip.to_string()),
            _ => None,
        })
        .expect("the player's LAN address");

    client.forward_rshell(&local_ip).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let child = format!(
        "mx = include_lib(\"/lib/metaxploit.so\"); wait(0.2); print mx.rshell_client(\"{public_ip}\", 1222, \"client\"); wait(1)"
    );
    let script = format!(
        r#"root = get_shell("root", "test")
pc = root.host_computer
pc.touch("/root", "rshell-client.src")
pc.File("/root/rshell-client.src").set_content({})
root.build("/root/rshell-client.src", "/root")
root.launch("/root/rshell-client")
mx = include_lib("/lib/metaxploit.so")
print "server ready"
shells = []
for i in range(0, 499)
  shells = mx.rshell_server
  if shells.len > 0 then break
  wait(0.02)
end for
print shells.len
if shells.len > 0 then print typeof(shells[0])"#,
        script_string(&child)
    );

    let outcomes = compare_in_real_time(&client, &user, &snapshot, &[script]).await;

    assert_succeeded(&outcomes);
    assert!(
        outcomes[0]
            .output
            .as_deref()
            .is_some_and(|output| output.trim_end().ends_with("shell")),
        "the client did not connect: {outcomes:?}"
    );
    client.shutdown();
}

/// A client another program connects while the server's program waits is in the server's list: both read the router's
/// reverse shells as the game saved them (`MetaExploitIntrinsics.rshell_client`, `RouterConfigOS.AddReverseShell`,
/// `MetaExploitIntrinsics.rshell_server`).
#[tokio::test]
async fn should_match_intrinsics_for_a_reverse_shell_from_another_program() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = rshell_server_network(&client).await;
    let server = r#"mx = include_lib("/lib/metaxploit.so")
print "before " + mx.rshell_server.len
wait(2)
print "after " + mx.rshell_server.len"#;
    let connecting = format!(
        r#"wait(0.5)
mx = include_lib("/lib/metaxploit.so")
print mx.rshell_client("{}", 1222, "client")
wait(4)"#,
        snapshot.public_ip
    );

    compare_pair_in(&client, &user, &snapshot, [server, &connecting]).await;
    client.shutdown();
}

/// A sniffer waits in one context while another logs in to SSH, capturing the login and saving its encoder.
#[tokio::test]
async fn should_match_intrinsics_for_a_concurrent_sniffer() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        find_service_account(snapshot, "ssh").is_some()
    })
    .await;
    let account = service_account(&snapshot, "ssh");
    let sniffing = r#"mx = include_lib("/lib/metaxploit.so")
print mx.sniffer(1)"#;
    let login = format!(
        "wait(0.2); print typeof(get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\"))",
        account.address, account.port, account.user, account.password
    );

    compare_pair_in(&client, &user, &snapshot, [sniffing, &login]).await;
    client.shutdown();
}
