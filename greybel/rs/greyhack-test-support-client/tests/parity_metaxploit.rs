//! metaxploit: libraries, net sessions, overflows and their requirements, reverse shells and sniffers.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Requirement, Reward};
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{
    dual_service_network, join_fixture, join_network, router_ssh_network, setup_player,
    ssh_network, test_client,
};
use crate::support::metaxploit::{
    computer_library, fixture_reply_weakness, fixture_weakness, overflow_script, player_libraries,
    router_libraries, router_library, weakness,
};
use crate::support::mirror::library;
use crate::support::parity::{
    assert_succeeded, compare_in, compare_in_any_user, compare_in_real_time, compare_pair_in,
    game_outcomes, snapshot_outcomes, without_users,
};
use crate::support::remote::{
    fixture_account, rshell_server_network, rshell_server_network_at, script_string,
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
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let forwarded = account.port;
    let snapshot = join_fixture(&client, &network).await;
    let mut targets = snapshot
        .nodes
        .iter()
        .filter_map(|node| match &node.node {
            LanNodeKind::Router(router) => library(&router.kernel_content)
                .is_some()
                .then(|| (node.local_ip.clone(), 0)),
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
                .map(|port| (node.local_ip.clone(), port.number)),
            LanNodeKind::Hub => None,
        })
        .collect::<Vec<_>>();

    targets.push((snapshot.public_ip.clone(), forwarded));

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
        .clone();
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
    let mut requested = vec![
        fixture_weakness(Reward::Shell, &[], "normal_user", ""),
        fixture_weakness(Reward::Computer, &[], "", ""),
        fixture_weakness(Reward::RandomFolder, &[], "", ""),
        fixture_weakness(Reward::ChangePass, &[], "", ""),
    ];

    requested.extend(
        [
            Requirement::Library,
            Requirement::NumberUsersRegister,
            Requirement::AnyActiveUser,
            Requirement::RootActiveUser,
            Requirement::GuestActiveUser,
            Requirement::PathExist,
        ]
        .into_iter()
        .map(|requirement| fixture_weakness(Reward::Shell, &[requirement], "", "")),
    );

    let installed = client
        .install_fixture_library("", "metaxploit.so", requested, false)
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let (_, library) = player_libraries(&snapshot)
        .into_iter()
        .find(|(path, _)| path == "/lib/metaxploit.so")
        .unwrap();
    let mut scripts = vec![
        "mx = include_lib(\"/lib/metaxploit.so\"); lib = mx.load(\"/lib/metaxploit.so\"); print lib.overflow(\"0xBAD\", \"missing\"); print lib.overflow(1, \"missing\")"
            .to_owned(),
    ];

    for fixture in &installed.weaknesses {
        let (address, weakness) = fixture_reply_weakness(&library, fixture);

        scripts.push(overflow_script(
            "/lib/metaxploit.so",
            None,
            address,
            weakness,
            "newpass123",
        ));
    }

    assert_eq!(
        scripts.len(),
        11,
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
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let ssh_ip = account.address;
    let ssh_port = account.port;

    join_fixture(&client, &network).await;

    let mut requested = vec![
        fixture_weakness(Reward::Shell, &[], "guest", ""),
        fixture_weakness(Reward::Computer, &[], "guest", ""),
        fixture_weakness(Reward::RandomFolder, &[], "guest", ""),
    ];

    requested.extend(
        [
            Requirement::Library,
            Requirement::NumberUsersRegister,
            Requirement::AnyActiveUser,
            Requirement::RootActiveUser,
            Requirement::PortForward,
            Requirement::GuestActiveUser,
            Requirement::PathExist,
        ]
        .into_iter()
        .map(|requirement| fixture_weakness(Reward::Shell, &[requirement], "guest", "")),
    );

    let installed = client
        .install_fixture_library(&ssh_ip, "libssh.so", requested, false)
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let library = computer_library(&snapshot, &ssh_ip, "libssh.so");
    let scripts: Vec<String> = installed
        .weaknesses
        .iter()
        .map(|fixture| {
            let (address, weakness) = fixture_reply_weakness(&library, fixture);

            overflow_script("", Some((&ssh_ip, ssh_port)), address, weakness, "")
        })
        .collect();

    assert!(installed.weaknesses.iter().any(|weakness| {
        weakness
            .requirements
            .contains(&u8::try_from(Requirement::PortForward.code()).unwrap())
    }));
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
    let network = router_ssh_network(&client, true).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let ssh_ip = account.address;
    let initial = join_fixture(&client, &network).await;
    let routers: Vec<String> = initial
        .nodes
        .iter()
        .filter(|node| matches!(node.node, LanNodeKind::Router(_)))
        .map(|node| node.local_ip.clone())
        .collect();
    let requested = vec![
        fixture_weakness(Reward::Shell, &[], "guest", ""),
        fixture_weakness(Reward::Computer, &[], "guest", ""),
        fixture_weakness(Reward::FirewallDisable, &[], "guest", ""),
        fixture_weakness(Reward::RandomFolder, &[Requirement::LocalNetwork], "", ""),
        fixture_weakness(Reward::RandomFolder, &[Requirement::ConnGateway], "", ""),
    ];
    let installed = client
        .install_fixture_library(&routers[0], "kernel_router.so", requested, false)
        .await
        .unwrap();

    // Use the same content at this version on every router so each net session dumps the requested zones.
    for router in routers.iter().skip(1) {
        client
            .write_router_kernel(router, &installed.content)
            .await
            .unwrap();
    }

    let snapshot = client.network_state().await.unwrap();
    let mut scripts = Vec::new();
    let mut firewall_scripts = Vec::new();
    let mut firewall = false;

    for ip in &routers {
        let library = router_library(&snapshot, ip);

        for fixture in &installed.weaknesses {
            let (address, weakness) = fixture_reply_weakness(&library, fixture);
            let script = overflow_script(
                "",
                Some((ip, 0)),
                address,
                weakness,
                if weakness.reward == Reward::FirewallDisable {
                    ""
                } else {
                    &ssh_ip
                },
            );

            if weakness.reward == Reward::FirewallDisable {
                if !firewall {
                    firewall = true;
                    firewall_scripts.push(format!(
                        "{script}; print get_router(\"{ip}\").firewall_rules"
                    ));
                }
            } else {
                scripts.push(script);
            }
        }
    }

    scripts.extend(firewall_scripts);
    assert!(routers.len() >= 3);
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

    let network = router_ssh_network(&client, false).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let ssh_ip = account.address;
    let initial = join_fixture(&client, &network).await;
    let main = initial.nodes[0].local_ip.clone();
    let lan = router_libraries(&initial)
        .into_iter()
        .map(|(ip, _)| ip)
        .find(|ip| ip != &main)
        .expect("a LAN router with a kernel library");

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
    let public_ip = before.public_ip.clone();
    let failures = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
print mx.rshell_client("bad", 1222, "client")
print mx.rshell_client("240.0.0.1", 1222, "client")
print mx.rshell_client("{public_ip}", 9999, "client")
print mx.rshell_client("{public_ip}", 1222, "bad name")
print mx.rshell_client("{public_ip}", 1222, "1234567890123456789012345")
print mx.rshell_client("{public_ip}", 1222, "kernel_task")
print mx.rshell_remove
print mx.rshell_remove(1)
print mx.rshell_server"#
    );

    compare_in(&client, &user, &before, &[failures]).await;

    let install = format!(
        "root = get_shell(\"root\", \"test\"); pc = root.host_computer; pc.touch(\"/root\", \"rshell-install.src\"); pc.File(\"/root/rshell-install.src\").set_content({}); print root.build(\"/root/rshell-install.src\", \"/root\"); print root.launch(\"/root/rshell-install\")",
        script_string("rshell = include_lib(\"/lib/librshell.so\"); print rshell.install_service")
    );

    compare_in(&client, &user, &before, &[install]).await;

    let installed = client.network_state().await.unwrap();
    let no_forward = r#"mx = include_lib("/lib/metaxploit.so")
print mx.rshell_server
print mx.rshell_remove
print mx.rshell_remove(1)"#
        .to_owned();

    compare_in(&client, &user, &installed, &[no_forward]).await;

    let local_ip = installed
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Computer(computer) if computer.player => Some(node.local_ip.clone()),
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
    let snapshot = rshell_server_network(&client, "RentServer").await;
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

/// Sixteen clients fit on one port; the seventeenth is refused. Removal validates lists, removes selected and all
/// shells, and listing after a client process ends prunes its registration (`RouterConfigOS`,
/// `MetaExploitIntrinsics.rshell_remove`).
#[tokio::test]
async fn should_match_intrinsics_for_reverse_shell_limits_removal_and_pruning() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = rshell_server_network(&client, "RentServer").await;
    let script = format!(
        r#"mx = include_lib("/lib/metaxploit.so")
print mx.rshell_remove
results = []
for i in range(1, 17)
  results.push(mx.rshell_client("{}", 1222, "client"))
end for
print results
shells = []
for i in range(0, 49)
  shells = mx.rshell_server
  if shells.len == 16 then break
  wait(0.02)
end for
print shells.len
print mx.rshell_remove(1)
print mx.rshell_remove([get_shell])
print mx.rshell_remove([shells[0], 1])
print mx.rshell_remove([shells[0]])
print mx.rshell_remove([shells[0]])
print mx.rshell_server.len
print mx.rshell_remove(null)
print mx.rshell_remove([])
print mx.rshell_server.len
print mx.rshell_client("{}", 1222, "stale")
stale_shells = []
for i in range(0, 49)
  stale_shells = mx.rshell_server
  if stale_shells.len == 1 then break
  wait(0.02)
end for
print stale_shells.len
pc = get_shell.host_computer
for line in pc.show_procs.split(char(10))[1:]
  parts = line.split(" ")
  if parts[-1] == "stale" then print pc.close_program(parts[1].to_int)
end for
print mx.rshell_server.len
print mx.rshell_remove"#,
        snapshot.public_ip, snapshot.public_ip
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// A CTF rshell service refuses removal with the network error, after the online check.
#[tokio::test]
async fn should_match_intrinsics_for_reverse_shell_removal_on_a_ctf_network() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = client.spawn_ctf_network().await.unwrap();
    let snapshot = rshell_server_network_at(&client, &network.public_ip).await;
    let script = "mx = include_lib(\"/lib/metaxploit.so\")\nprint mx.rshell_remove(1)".to_owned();

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// Pending SSH encryption rejects rshell removal before any world or service lookup.
#[tokio::test]
async fn should_match_intrinsics_for_reverse_shell_removal_during_ssh_encryption() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = dual_service_network(&client).await;
    let account = fixture_account(&network, "dual", "ssh", true);
    let address = network.public_ip.clone();
    let port = account.port;
    let password = account.root_password;
    let snapshot = join_fixture(&client, &network).await;
    let connect =
        format!("get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\")");
    let encode = script_string(
        "Encode = function(password)\n  print include_lib(\"/lib/metaxploit.so\").rshell_remove\n  return \"enc_\" + password\nend function",
    );
    let config = script_string(
        "{\"encryption_enabled\":true,\"path_enc\":\"/server/conf/encode.src\",\"path_dec\":\"/server/conf/decode.bin\"}",
    );
    let script = format!(
        "remote = {connect}; pc = remote.host_computer; pc.touch(\"/server/conf\", \"encode.src\"); pc.File(\"/server/conf/encode.src\").set_content({encode}); pc.File(\"/server/conf/sshd.conf\").set_content({config}); print {connect}"
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// A sniffer waits in one context while another logs in to SSH, capturing the login and saving its encoder.
#[tokio::test]
async fn should_match_intrinsics_for_a_concurrent_sniffer() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let snapshot = join_fixture(&client, &network).await;
    let sniffing = r#"mx = include_lib("/lib/metaxploit.so")
print mx.sniffer(1)"#;
    let login = format!(
        "wait(0.2); print typeof(get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\"))",
        account.address, account.port, account.user, account.password
    );

    compare_pair_in(&client, &user, &snapshot, [sniffing, &login]).await;
    client.shutdown();
}
