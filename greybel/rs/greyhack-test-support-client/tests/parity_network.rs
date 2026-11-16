//! Networks: the player's network, reaching addresses and services, routers, ports, DNS, FTP and network sessions.

use greybel_test_support::game::Game;
use greyhack_behaviour::{Reward, WorldSeed};
use greyhack_test_support_client::GuildSetup;
use greyhack_test_support_client::protocol::test_support::{
    FixtureService, FixtureTarget, LanNodeKind,
};
use greyhack_world_creation::WifiGenerator;

use crate::support::game::{
    devices_above, dual_service_network, empty_network, join_fixture, join_network,
    routed_ssh_network, router_ssh_network, service_network, setup_player, ssh_network,
    ssh_network_of_kind, test_client,
};
use crate::support::metaxploit::{
    fixture_reply_weakness, fixture_weakness, overflow_script, router_library,
};
use crate::support::parity::{
    assert_port_info_lines, assert_succeeded, compare_in, compare_in_flooded,
    compare_in_flooded_labeled, compare_labeled, game_outcomes, intrinsics_outcomes,
    snapshot_outcomes,
};
use crate::support::parity_bank_payments::rent_network;
use crate::support::parity_guilds::LEVEL_FIVE;
use crate::support::parity_missions::use_game_word_lists;
use crate::support::remote::{fixture_account, script_string};

/// Public addresses nothing reached yet: the game makes a network the first time a script reaches one, outside the
/// reserved ranges.
#[tokio::test]
async fn should_match_intrinsics_for_addresses_reached_for_the_first_time() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "sh = get_shell; print sh.ping(\"1.2.3.4\"); print sh.ping(\"100.64.0.1\"); print sh.ping(\"240.0.0.1\"); print sh.ping(\"0.1.2.3\")",
        "print get_router(\"5.6.7.8\") != null; print get_router(\"5.6.7.8\").public_ip; print get_router(\"100.64.0.1\"); print get_router(\"224.1.1.1\")",
        "print typeof(whois(\"9.10.11.12\")); print whois(\"100.64.0.1\"); print whois(\"198.18.0.1\")",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// `whois` on a rented network with its administrator contact block.
#[tokio::test]
async fn should_match_intrinsics_for_whois_variants() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let rented = rent_network(&client, std::process::id() % 10000).await;

    client.hire_rental_admin(&rented).await.unwrap();

    let snapshot = client.network_state().await.unwrap();

    assert!(
        snapshot.rental_admin,
        "the rental administrator fixture is active"
    );
    compare_in(
        &client,
        &user,
        &snapshot,
        &[format!("print whois(\"{rented}\")")],
    )
    .await;
    client.shutdown();
}

/// A reachable public router can still be offline or fail its own startup checks.
#[tokio::test]
async fn should_match_intrinsics_for_router_host_errors() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let current = ssh_network(&client).await;

    for fixture in ["disabled", "startup_error"] {
        let target = empty_network(&client, "Supermercados").await;
        let target_ip = target.public_ip.clone();

        join_fixture(&client, &target).await;
        if fixture == "disabled" {
            client.disable_network(&target_ip, true).await.unwrap();
        } else {
            client
                .configure_computer(&target.main_local_ip, "startup_error")
                .await
                .unwrap();
        }

        join_fixture(&client, &current).await;

        let snapshot = client.network_state().await.unwrap();

        compare_in(
            &client,
            &user,
            &snapshot,
            &[format!("print get_router(\"{target_ip}\")")],
        )
        .await;
    }

    client.shutdown();
}

/// Fresh networks draw their public Wi-Fi names from the world secret, independently of the snapshot mirror
/// (`ServerMap.RouterNode`, `NetworkLan.RouterNet.ConfigWifi`).
#[tokio::test]
async fn should_generate_public_wifi_names_from_the_world_secret() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    use_game_word_lists(&client).await;
    setup_player(&client).await;

    let mut comparisons = 0;

    for _ in 0..3 {
        let network = router_ssh_network(&client, false).await;
        let snapshot = join_fixture(&client, &network).await;
        let seed = WorldSeed::new(snapshot.world_secret.to_vec());
        let generator = WifiGenerator::new(&seed);
        let before = comparisons;

        for node in &snapshot.nodes {
            let LanNodeKind::Router(router) = &node.node else {
                continue;
            };
            let (Some(bssid), Some(essid)) = (&router.bssid, &router.essid) else {
                continue;
            };
            let public_ip = snapshot.public_ip.clone();
            let local_ip = node.local_ip.clone();
            let addresses = if node.parent.is_some() {
                vec![public_ip.as_str(), local_ip.as_str()]
            } else {
                vec![public_ip.as_str()]
            };
            let (expected_bssid, expected_essid) = generator.names(&addresses);

            assert_eq!(
                (bssid.clone(), essid.clone()),
                (expected_bssid, expected_essid),
                "Wi-Fi names of {public_ip} / {local_ip}"
            );
            comparisons += 1;
        }

        assert!(
            comparisons >= before + 2,
            "each network compares a main and a LAN router"
        );
    }

    assert!(comparisons >= 6, "three networks have Wi-Fi comparisons");
    client.shutdown();
}

/// The player's network, the simulated one laid out like the game's: its main router, LAN router and switch, what
/// they list and forward, what answers a ping and what a connection meets on the way.
#[tokio::test]
async fn should_match_intrinsics_for_the_players_network() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = router_ssh_network(&client, true).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let computer = account.address;
    let forwarded_port = account.port;
    let snapshot = join_fixture(&client, &network).await;
    let child_router = snapshot
        .nodes
        .iter()
        .find(|node| {
            node.parent.is_some()
                && matches!(&node.node, LanNodeKind::Router(router) if router.kind == "ROUTER")
        })
        .expect("the network has a LAN router")
        .local_ip
        .clone();
    let switch = snapshot
        .nodes
        .iter()
        .find(|node| matches!(&node.node, LanNodeKind::Router(router) if router.kind == "SWITCH"))
        .expect("the network has a switch")
        .local_ip
        .clone();
    let mut addresses: Vec<String> = snapshot
        .nodes
        .iter()
        .filter(|node| !node.local_ip.is_empty())
        .map(|node| node.local_ip.clone())
        .collect();

    addresses.push("10.0.0.1".to_owned());

    let service_addresses = snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };

            computer
                .ports
                .iter()
                .find(|port| port.service == "ssh" && !port.closed)
                .map(|port| (node.local_ip.clone(), port.number))
        })
        .collect::<Vec<_>>();
    let mut scripts = vec![
        "r = get_router; print [r.public_ip, r.local_ip, r.essid_name, r.bssid_name, r.kernel_version]; print r.devices_lan_ip; print r.firewall_rules".to_owned(),
        format!("r = get_router; for p in r.used_ports; print [p.port_number, p.is_closed, p.get_lan_ip, r.port_info(p)]; end for; print r.ping_port({forwarded_port}).get_lan_ip; print r.ping_port(8080)"),
        format!("r = get_router(\"{child_router}\"); print [r.local_ip, r.public_ip, r.essid_name, r.bssid_name, r.kernel_version, r.used_ports.len]; print r.devices_lan_ip; print r.firewall_rules"),
        format!("s = get_switch(\"{switch}\"); print [typeof(s), s.local_ip, s.essid_name, s.bssid_name]; print s.firewall_rules; print s.devices_lan_ip; print get_switch(\"{child_router}\"); print get_router(\"{computer}\"); print get_router(\"{switch}\").local_ip"),
        format!("sh = get_shell; for a in {addresses:?}; print a + \" \" + sh.ping(a); end for"),
        format!("r = get_router; for a in {addresses:?}; ports = r.device_ports(a); if typeof(ports) == \"string\" then print ports else print a + \" \" + ports.len; end for"),
    ];

    for (address, port) in service_addresses {
        scripts.push(format!(
            "sh = get_shell; print sh.connect_service(\"{address}\", {port}, \"root\", \"x\")"
        ));
    }

    scripts.push(format!(
        "sh = get_shell; print sh.connect_service(\"{}\", {forwarded_port}, \"root\", \"x\"); print sh.connect_service(\"10.0.0.1\", 22, \"root\", \"x\")",
        snapshot.public_ip
    ));

    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = snapshot_outcomes(&scripts, &user, &snapshot);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// A service behind a router or switch that doesn't start can't be reached (`Networking.CheckServiceOnline`,
/// `Computer.IsOutGoingConnectionOk`): `connect_service` returns the game's message.
#[tokio::test]
async fn should_match_intrinsics_for_a_service_behind_an_unavailable_lan_router() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    // `connect_service`'s default service is SSH; an FTP port would fail the port check first.
    let network = routed_ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);
    let host = account.address;
    let port = account.port;
    let password = account.root_password;
    let snapshot = join_fixture(&client, &network).await;
    let devices = devices_above(&snapshot, &host);

    assert!(!devices.is_empty());

    // The firewalls on the way would refuse the connection before the devices are checked. The next player setup
    // puts both back.
    for device in &devices {
        client.allow_firewall(device).await.unwrap();
        client.write_router_kernel(device, "{}").await.unwrap();
    }

    let snapshot = client.network_state().await.unwrap();
    let script = format!(
        "remote = get_shell.connect_service(\"{host}\", {port}, \"root\", \"{password}\"); print typeof(remote); print typeof(get_router); print remote"
    );
    let outcomes = compare_in(&client, &user, &snapshot, &[script]).await;

    assert_eq!(
        outcomes[0].output.as_deref(),
        Some(
            "string\nrouter\nUnable to connect: The target is not accessible due to a remote network connection error."
        )
    );
    client.shutdown();
}

/// FTP shells upload and download through `scp`, with the game's path and permission failures.
#[tokio::test]
async fn should_match_intrinsics_for_ftp_transfers() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let police = service_network(&client, "Comisaria", "ftp", 10_021, 10_021).await;
    let account = fixture_account(&police, "ftp", "ftp", false);
    let snapshot = join_fixture(&client, &police).await;
    let address = &account.address;
    let port = account.port;
    let root = &account.root_password;
    let scripts = vec![
        format!(
            "local = get_shell; pc = local.host_computer; pc.touch(home_dir, \"ftp-upload.txt\"); pc.File(home_dir + \"/ftp-upload.txt\").set_content(\"from player\"); ftp = local.connect_service(\"{address}\", {port}, \"root\", \"{root}\", \"ftp\"); print typeof(ftp); print local.scp(home_dir + \"/ftp-upload.txt\", \"/root\", ftp, 1); print ftp.host_computer.File(\"/root/ftp-upload.txt\") != null; print ftp.scp(\"/root/ftp-upload.txt\", home_dir, local); print pc.File(home_dir + \"/ftp-upload.txt\").get_content"
        ),
        format!(
            "local = get_shell; ftp = local.connect_service(\"{address}\", {port}, \"root\", \"{root}\", \"ftp\"); print ftp.scp(\"/root/missing.txt\", home_dir, local); print local.scp(home_dir + \"/missing.txt\", \"/root\", ftp, 1); print ftp.scp(\"/etc/passwd\", \"/missing\", local); print local.scp(home_dir + \"/ftp-upload.txt\", \"/etc/passwd\", ftp, 1)"
        ),
        format!(
            "local = get_shell; ftp = local.connect_service(\"{address}\", {port}, \"{}\", \"{}\", \"ftp\"); print typeof(ftp); print local.scp(home_dir + \"/ftp-upload.txt\", \"/root\", ftp, 1); print ftp.scp(\"/etc/shadow\", home_dir, local)",
            account.user, account.password
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// `scp` and `build` argument, permission, source-type, and destination-creation failures.
#[tokio::test]
async fn should_match_intrinsics_for_scp_and_build_edges() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client.configure_computer("", "permissions").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let prelude = r#"root = get_shell("root", "test")
limited = get_shell("limited", "pass1")
pc = root.host_computer
b = "/home/limited/matrix"
pc.touch(b + "/open", "code.src")
pc.File(b + "/open/code.src").set_content("print 1")
root.build(b + "/open/code.src", b + "/open")
pc.touch(b + "/open", "plain")
pc.File(b + "/open/plain").set_content("print 1")
pc.touch(b + "/open", "empty.src")
pc.File(b + "/open/empty.src").set_content("")
"#;
    let scripts = vec![
        "print scp(1, \"/etc/passwd\", home_dir, get_shell); print get_shell.scp(1, home_dir, get_shell); print get_shell.scp(\"\", home_dir, get_shell); print get_shell.scp(\"/etc/passwd\", \"\", get_shell); print get_shell.scp(\"/etc/passwd\", home_dir, {})".to_owned(),
        "print limited.scp(b + \"/locked.txt\", b + \"/open\", root); print limited.scp(b + \"/locked\", b + \"/open\", root); print root.scp(\"/\", home_dir, root); print limited.scp(b + \"/mixed\", b + \"/destination\", root)".to_owned(),
        "print build(1, home_dir, home_dir); print build({}, home_dir, home_dir); print root.build(\"\", home_dir); print root.build(home_dir, \"\")".to_owned(),
        "print root.build(b + \"/missing.src\", b + \"/open\"); print root.build(b + \"/open/code.src\", b + \"/missing\"); print limited.build(b + \"/locked.txt\", b + \"/open\"); print limited.build(b + \"/open/code.src\", b + \"/destination_locked\")".to_owned(),
        "print root.build(b + \"/open/code\", b + \"/open\"); print root.build(b + \"/open/plain\", b + \"/open\"); print root.build(b + \"/open/empty.src\", b + \"/open\")".to_owned(),
        "print limited.launch(\"/etc/passwd\"); pc.File(b + \"/open/code\").chmod(\"o-x\"); print limited.launch(b + \"/open/code\"); print root.launch(b + \"/open/code\", \"[\")".to_owned(),
        "print get_shell.ping(1); print get_shell.ping(\"bad\"); print ping({}, \"1.2.3.4\"); get_shell.host_computer.File(\"/lib/init.so\").delete; print get_shell.ping(\"1.2.3.4\")".to_owned(),
    ]
    .into_iter()
    .map(|script| format!("{prelude}\n{script}"))
    .collect::<Vec<_>>();

    compare_in(&client, &user, &snapshot, &scripts).await;

    for fixture in ["full_disk", "full_filesystem"] {
        let user = setup_player(&client).await;

        client.configure_computer("", fixture).await.unwrap();

        let snapshot = client.network_state().await.unwrap();
        let destination = if fixture == "full_disk" {
            "home_dir".to_owned()
        } else {
            "home_dir + \"/full_filesystem\"".to_owned()
        };
        let scripts = vec![format!(
            "print get_shell.scp(\"/etc/passwd\", {destination}, get_shell); print get_shell.build(\"/etc/passwd\", {destination})"
        )];

        compare_in(&client, &user, &snapshot, &scripts).await;
    }

    client.shutdown();
}

/// SSH and FTP run a reached computer's encode/decode scripts for successful and failed logins, retry a changed
/// decoded password, and honour the encryption message (`ShellIntrinsics.connect_service`, `PlayerUtils.ConnectService`).
#[tokio::test]
async fn should_match_intrinsics_for_encrypted_service_logins() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = dual_service_network(&client).await;
    let address = network.public_ip.clone();
    let ssh = fixture_account(&network, "dual", "ssh", true);
    let ftp = fixture_account(&network, "dual", "ftp", true);
    let port = ssh.port;
    let password = ssh.root_password;
    let ftp_port = ftp.port;
    let ftp_password = ftp.root_password;
    let snapshot = join_fixture(&client, &network).await;
    let encode =
        script_string("Encode = function(password)\n  return \"enc_\" + password\nend function");
    let decode = script_string(&format!(
        "Decode = function(password)\n  if password == \"enc_wrong\" then return \"{password}\"\n  return password[4:]\nend function"
    ));
    let config = |message| {
        script_string(&format!(
            "{{\"encryption_enabled\":true,\"message_encrypted_conn\":{message},\"path_enc\":\"/server/conf/encode.src\",\"path_dec\":\"/server/conf/decode.bin\"}}"
        ))
    };
    let ftp_config = script_string(
        "{\"encryption_enabled\":true,\"message_encrypted_conn\":true,\"path_enc\":\"/server/conf/ftp_encode.src\",\"path_dec\":\"/server/conf/ftp_decode.bin\"}",
    );
    let connect = |password: &str| {
        format!("get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\")")
    };
    let script = format!(
        "remote = {connect}; pc = remote.host_computer; pc.touch(\"/server/conf\", \"encode.src\"); \
         pc.File(\"/server/conf/encode.src\").set_content({encode}); pc.touch(\"/server/conf\", \"decode.bin.src\"); \
         pc.File(\"/server/conf/decode.bin.src\").set_content({decode}); print remote.build(\"/server/conf/decode.bin.src\", \"/server/conf\"); \
         pc.File(\"/server/conf/sshd.conf\").set_content({enabled}); print \"correct\"; print typeof({correct}); \
         print \"wrong\"; print {wrong}; print \"decoded retry\"; print typeof({retry}); \
         pc.File(\"/server/conf/encode.src\").set_content(\"print(\"\"encoder broke\"\")\"); print \"encode failure\"; \
         print {correct}; pc.File(\"/server/conf/encode.src\").set_content({encode}); \
         pc.File(\"/server/conf/sshd.conf\").set_content({silent}); print \"silent\"; print typeof({correct}); \
         pc.touch(\"/server/conf\", \"ftp_encode.src\"); pc.File(\"/server/conf/ftp_encode.src\").set_content({encode}); \
         pc.touch(\"/server/conf\", \"ftp_decode.bin.src\"); pc.File(\"/server/conf/ftp_decode.bin.src\").set_content({decode}); \
         print remote.build(\"/server/conf/ftp_decode.bin.src\", \"/server/conf\"); \
         pc.File(\"/server/conf/sshd.conf\").set_content({ftp_config}); print remote.scp(\"/server/conf/ftp_encode.src\", home_dir, get_shell); \
         print remote.scp(\"/server/conf/ftp_decode.bin\", home_dir, get_shell); print remote.scp(\"/server/conf/sshd.conf\", home_dir, get_shell); \
         ftp_remote = get_shell.connect_service(\"{address}\", {ftp_port}, \"root\", \"{ftp_password}\", \"ftp\"); \
         print get_shell.scp(home_dir + \"/ftp_encode.src\", \"/server/conf\", ftp_remote); \
         print get_shell.scp(home_dir + \"/ftp_decode.bin\", \"/server/conf\", ftp_remote); \
         print get_shell.scp(home_dir + \"/sshd.conf\", \"/server/conf\", ftp_remote); \
         print \"ftp\"; print typeof(get_shell.connect_service(\"{address}\", {ftp_port}, \"root\", \"{ftp_password}\", \"ftp\"))",
        connect = connect(&password),
        enabled = config(true),
        silent = config(false),
        correct = connect(&password),
        wrong = connect("bad"),
        retry = connect("wrong"),
    );

    let outcomes = compare_in(&client, &user, &snapshot, &[script]).await;

    assert!(
        outcomes[0]
            .output
            .as_deref()
            .is_some_and(|output| output.contains("ftp\nEncryption enabled...\nftpshell")),
        "FTP must complete an encrypted login: {:?}",
        outcomes[0]
    );
    client.shutdown();
}

/// `connect_service` validates its arguments and the source computer before it resolves the target.
#[tokio::test]
async fn should_match_intrinsics_for_connection_argument_and_source_errors() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = dual_service_network(&client).await;
    let public = fixture_account(&network, "dual", "ssh", true);
    let local = fixture_account(&network, "dual", "ssh", false);
    let ftp = fixture_account(&network, "dual", "ftp", false);
    let address = public.address.clone();
    let port = public.port;
    let password = public.root_password.clone();
    let snapshot = join_fixture(&client, &network).await;
    let scripts = vec![
        "print connect_service(1, \"1.2.3.4\", 22, \"root\", \"x\"); print get_shell.connect_service(\"\", 22, \"root\", \"x\"); print get_shell.connect_service(\"1.2.3.4\", \"22\", \"root\", \"x\")".to_owned(),
        "print get_shell.connect_service(\"1.2.3.4\", 22, \"root\", \"[\")".to_owned(),
        format!("print get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\", \"http\")"),
        "print get_shell.connect_service(\"no-such-domain.invalid\", 22, \"root\", \"x\"); print get_shell.connect_service(\"10.0.0.1\", 22, \"root\", \"x\")".to_owned(),
        format!("print get_shell.connect_service(\"{address}\", 1, \"root\", \"{password}\"); print get_shell.connect_service(\"{}\", 1, \"root\", \"{}\")", local.address, local.root_password),
        format!("print get_shell.connect_service(\"{}\", {}, \"root\", \"{}\", \"ssh\")", local.address, ftp.port, local.root_password),
        format!("s = get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\"); s.host_computer.File(\"/lib/init.so\").delete; print s.connect_service(\"{address}\", {port}, \"root\", \"{password}\")"),
        format!("get_shell.host_computer.File(\"/lib/init.so\").delete; print get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\"); print get_shell.ping(\"{address}\")"),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// Public-forward, target-availability, firewall, and service-library errors in their game validation order.
#[tokio::test]
async fn should_match_intrinsics_for_connection_target_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for fixture in [
        "closed_public_service",
        "missing_machine",
        "unreachable",
        "rebooting",
        "firewall_deny",
    ] {
        let user = setup_player(&client).await;
        let network = ssh_network(&client).await;
        let public = fixture_account(&network, "ssh", "ssh", true);
        let local = fixture_account(&network, "ssh", "ssh", false);

        join_fixture(&client, &network).await;
        client
            .configure_computer(&local.address, fixture)
            .await
            .unwrap();

        let snapshot = client.network_state().await.unwrap();
        let script = format!(
            "print get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); print get_shell.ping(\"{}\")",
            public.address, public.port, public.root_password, public.address
        );

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("connection-target:{fixture}"),
            &[script],
        )
        .await;
    }

    let user = setup_player(&client).await;
    let network = client
        .create_fixture_network(
            "RentServer",
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
                        service: "ftp".to_owned(),
                        internal_port: 10_021,
                        public_port: Some(10_021),
                        oldest_library: false,
                    },
                    FixtureService {
                        service: "ssh".to_owned(),
                        internal_port: 10_022,
                        public_port: Some(10_022),
                        oldest_library: false,
                    },
                ],
            }],
        )
        .await
        .unwrap();
    let ftp = fixture_account(&network, "dual", "ftp", true);
    let local = fixture_account(&network, "dual", "ftp", false);

    join_fixture(&client, &network).await;
    client
        .configure_computer(&local.address, "missing_ssh_service")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();

    assert!(
        snapshot.admin.is_none(),
        "connection-target:missing_ssh_service"
    );

    let script = format!(
        "print get_shell.connect_service(\"{}\", {}, \"root\", \"{}\")",
        ftp.address, ftp.port, ftp.root_password
    );

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "connection-target:missing_ssh_service",
        &[script],
    )
    .await;

    client.shutdown();
}

/// Each service-library fault starts with one valid original library and no NPC administrator to restore it.
#[tokio::test]
async fn should_match_intrinsics_for_connection_library_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for fixture in ["missing", "invalid", "unexpected", "malformed"] {
        let user = setup_player(&client).await;
        let network = client
            .create_fixture_network(
                "RentServer",
                "flat",
                false,
                vec![FixtureTarget {
                    name: "ssh".to_owned(),
                    device: "COMPUTER".to_owned(),
                    location: "main".to_owned(),
                    root_password: String::new(),
                    library_state: fixture.to_owned(),
                    services: vec![FixtureService {
                        service: "ssh".to_owned(),
                        internal_port: 31_022,
                        public_port: Some(31_022),
                        oldest_library: false,
                    }],
                }],
            )
            .await
            .unwrap();
        let account = fixture_account(&network, "ssh", "ssh", true);

        join_network(&client, &network.public_ip).await;

        let snapshot = client.network_state().await.unwrap();

        assert!(
            snapshot.admin.is_none(),
            "library_state={fixture}: the fixture must not have an NPC administrator"
        );

        let connect = format!(
            "s = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); print typeof(s); if typeof(s) == \"shell\" then print s.host_computer.local_ip; if typeof(s) == \"shell\" then print s.host_computer.File(\"/lib/libssh.so\") == null",
            account.address, account.port, account.root_password
        );

        let expected = snapshot_outcomes(&[&connect], &user, &snapshot);
        let actual = game_outcomes(&client, &[&connect]).await;

        assert_eq!(actual, expected, "library_state={fixture}");
    }

    client.shutdown();
}

/// A router's own file system, reached as guest through its kernel's computer reward: the router layout, its website
/// and its kernel (`XmlGlobal.GenerarRouterFileSystem`, `MetaLibIntrinsics.overflow`, `Router.GetComputer`,
/// `RouterIntrinsics.kernel_version`).
#[tokio::test]
async fn should_match_intrinsics_for_a_routers_file_system() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let initial = client.network_state().await.unwrap();
    let address = initial.public_ip.clone();
    let main = initial.nodes[0].local_ip.clone();
    let installed = client
        .install_fixture_library(
            &main,
            "kernel_router.so",
            vec![fixture_weakness(Reward::Computer, &[], "guest", "")],
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let kernel = router_library(&snapshot, &main);
    let (zone, weakness) = fixture_reply_weakness(&kernel, &installed.weaknesses[0]);
    let routers: Vec<String> = snapshot
        .nodes
        .iter()
        .filter(|node| matches!(node.node, LanNodeKind::Router(_)))
        .map(|node| node.local_ip.clone())
        .collect();
    let scripts: Vec<String> = routers
        .iter()
        .map(|router| {
            let exploit = overflow_script("", Some((&main, 0)), zone, weakness, router);

            format!(
                "{exploit}; pc = lib.overflow(\"{zone}\", \"{unsec}\", \"{router}\"); \
                 print [pc.get_name, pc.local_ip, pc.public_ip]; \
                 for folder in [\"/\", \"/bin\", \"/usr/bin\", \"/home\", \"/home/guest\", \"/server\", \"/Public/htdocs\", \"/lib\", \"/etc\", \"/sys\", \"/boot\"]; \
                 f = pc.File(folder); names = []; if typeof(f) == \"file\" and typeof(f.get_files) == \"list\" then; \
                 for file in f.get_folders + f.get_files; names.push(file.name + \" \" + file.permissions + \" \" + file.owner + \" \" + file.size); end for; \
                 end if; print folder + \": \" + typeof(f) + \" \" + names.join(\", \"); end for; \
                 print pc.File(\"/Public/htdocs/website.html\").get_content; print typeof(pc.File(\"/lib/kernel_router.so\")); \
                 print get_router(\"{address}\").kernel_version; print get_router(\"{router}\").kernel_version",
                unsec = weakness.unsec_value
            )
        })
        .collect();

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// `port_info` reads the service and its library's version as the router finds them (`Router.GetPortVersion`), for
/// the router's forwards and a computer's own ports. A library a script renames counts only once the game saved the
/// script's change, which the comparison doesn't reach.
#[tokio::test]
async fn should_match_intrinsics_for_port_info() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let address = network.public_ip.clone();
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let password = account.root_password;

    let snapshot = join_fixture(&client, &network).await;
    let connect =
        format!("get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\")");
    let script = format!(
        "r = get_router(\"{address}\"); ports = r.used_ports; info = function(); out = []; \
         for p in ports; out.push(p.port_number + \" \" + r.port_info(p)); end for; return out.join(\", \"); end function; \
         print info; s = {connect}; pc = s.host_computer; \
         for p in pc.get_ports; print p.port_number + \" \" + r.port_info(p); end for; \
         print get_router.port_info(ports[0])"
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// Nothing from outside connects to a network disabled for not having been paid for, nor exploits it, while its own
/// LAN still reaches its computers (`NetworkLan.disabledNetwork`).
#[tokio::test]
async fn should_match_intrinsics_for_a_disabled_network() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let address = network.public_ip.clone();
    let public = fixture_account(&network, "ssh", "ssh", true);
    let port = public.port;
    let password = public.root_password;
    let local = fixture_account(&network, "ssh", "ssh", false);
    let lan_ip = local.address;
    let lan_port = local.port;
    let lan_password = local.root_password;

    join_network(&client, &address).await;
    client.disable_network(&address, true).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let script = format!(
        "print get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\")
mx = include_lib(\"/lib/metaxploit.so\")
m = mx.net_use(\"{address}\", {port})
print typeof(m)
if m then print m.dump_lib.overflow(\"0x0\", \"x\")
r = get_router(\"{address}\")
c = get_shell.connect_service(\"{lan_ip}\", {lan_port}, \"root\", \"{lan_password}\")
print \"lan \" + typeof(c)
if typeof(c) == \"shell\" then
    parts = r.local_ip.split(\"\\.\")
    print c.host_computer.connect_ethernet(\"eth0\", parts[0] + \".\" + parts[1] + \".\" + parts[2] + \".250\", r.local_ip)
end if
print get_shell.host_computer.is_network_active"
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// The counters and active-user checks of service and router net sessions, then the flood that reaches the reboot
/// threshold (`FloodSystem.AddFlood`). A root program launched on the computer first has ended by then, its process
/// with it; the network's administrator counts as an active root user while at work (`Computer.IsRootActiveUser`).
#[tokio::test]
async fn should_match_intrinsics_for_net_session_queries_and_flooding() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let before = join_fixture(&client, &network).await;

    assert!(before.admin.is_some());

    let account = fixture_account(&network, "ssh", "ssh", false);
    let main = before.nodes[0].local_ip.clone();

    client.prime_flood(&account.address).await.unwrap();
    client.prime_flood(&main).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let super_admin = snapshot.rental_admin
        || snapshot.admin.as_ref().is_some_and(|admin| {
            admin.work_performance > 2
                && u16::midpoint(
                    u16::from(admin.intelligence),
                    u16::from(admin.pc_experience),
                ) >= 8
                && !admin.dead_or_fired
        });
    let mut threshold = u32::from(snapshot.net_speed) * 3;

    if !super_admin {
        threshold = (threshold * 3).div_ceil(4);
    }

    let floods = [account.address.as_str(), main.as_str()]
        .into_iter()
        .flat_map(|address| {
            (1..threshold)
                .map(move |origin| (address.to_owned(), format!("fixture-{origin}"), threshold))
        })
        .collect::<Vec<_>>();
    let scripts = vec![format!(
        "shell = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); pc = shell.host_computer; pc.touch(\"/root\", \"active.src\"); pc.File(\"/root/active.src\").set_content(\"wait(1)\"); shell.build(\"/root/active.src\", \"/root\"); shell.launch(\"/root/active\"); mx = include_lib(\"/lib/metaxploit.so\"); session = mx.net_use(\"{}\", {}); print [session.get_num_users, session.get_num_portforward, session.get_num_conn_gateway, session.is_any_active_user, session.is_root_active_user]; router = mx.net_use(\"{}\", 0); print [router.get_num_users, router.get_num_portforward, router.get_num_conn_gateway, router.is_any_active_user, router.is_root_active_user]; print session.flood_connection; print router.flood_connection",
        account.address, account.port, account.root_password, account.address, account.port, main
    )];

    compare_in_flooded(&client, &user, &snapshot, &scripts, &floods).await;
    client.shutdown();
}

/// Wrong router, port and net-session receivers, including their distinct helper crashes.
#[tokio::test]
async fn should_match_intrinsics_for_router_port_and_session_receivers() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = vec![
        "print get_router.net_speed; print [essid_name({}), bssid_name({}), net_speed({}), firewall_rules({}), kernel_version({}), devices_lan_ip({}), used_ports({})]".to_owned(),
        "print [device_ports(get_router, \"\"), device_ports(get_router, 1), device_ports({}, \"10.0.0.1\"), ping_port(get_router, \"x\"), ping_port({}, 22)]".to_owned(),
        "print [get_lan_ip(1), is_closed(1)]".to_owned(),
        "print [dump_lib({}), get_num_users({}), get_num_portforward({}), get_num_conn_gateway({}), is_any_active_user({}), is_root_active_user({}), flood_connection({})]".to_owned(),
        "print get_lan_ip({})".to_owned(),
        "print is_closed({})".to_owned(),
    ];

    compare_labeled(&client, &user, &snapshot, "receiver-errors", &scripts).await;
    client.shutdown();
}

/// A router without its kernel has no version, and a service computer without ports returns an empty list.
#[tokio::test]
async fn should_match_intrinsics_for_router_kernel_and_device_port_edges() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for fixture in [
        "missing_kernel",
        "invalid_kernel",
        "portless",
        "out_of_network",
    ] {
        let user = setup_player(&client).await;
        let network = if fixture == "portless" {
            client
                .create_fixture_network(
                    "RentServer",
                    "flat",
                    false,
                    vec![FixtureTarget {
                        name: "portless".to_owned(),
                        device: "COMPUTER".to_owned(),
                        location: "main".to_owned(),
                        root_password: String::new(),
                        library_state: String::new(),
                        services: Vec::new(),
                    }],
                )
                .await
                .unwrap()
        } else {
            ssh_network(&client).await
        };

        join_fixture(&client, &network).await;

        if fixture == "missing_kernel" || fixture == "invalid_kernel" {
            client
                .configure_computer(&network.main_local_ip, fixture)
                .await
                .unwrap();
        }

        let other = if fixture == "out_of_network" {
            Some(ssh_network_of_kind(&client, "RentServer").await)
        } else {
            None
        };
        let snapshot = client.network_state().await.unwrap();
        let local = network.targets[0].local_ip.clone();
        let script = match fixture {
            "missing_kernel" | "invalid_kernel" => "print get_router.kernel_version".to_owned(),
            "portless" => format!("print get_router.device_ports(\"{local}\")"),
            "out_of_network" => format!(
                "r = get_router(\"{}\"); print r.device_ports(\"{local}\")",
                other.unwrap().public_ip
            ),
            _ => unreachable!(),
        };

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("router-device:{fixture}"),
            &[script],
        )
        .await;
    }

    client.shutdown();
}

/// A net session keeps its object after its service library disappears, but can no longer dump it.
#[tokio::test]
async fn should_match_intrinsics_for_a_stale_net_session_library() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for fixture in ["missing", "invalid"] {
        let user = setup_player(&client).await;
        let network = ssh_network_of_kind(&client, "RentServer").await;
        let account = fixture_account(&network, "ssh", "ssh", false);
        let snapshot = join_fixture(&client, &network).await;
        let action = if fixture == "missing" {
            "pc.File(\"/lib/libssh.so\").delete"
        } else {
            "pc.touch(\"/root\", \"notlib.txt\"); pc.File(\"/root/notlib.txt\").move(\"/lib\", \"libssh.so\")"
        };
        let script = format!(
            "remote = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); pc = remote.host_computer; mx = include_lib(\"/lib/metaxploit.so\"); session = mx.net_use(\"{}\", {}); {action}; print session.dump_lib",
            account.address, account.port, account.root_password, account.address, account.port
        );

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("stale-dump-lib:{fixture}"),
            &[script],
        )
        .await;
    }

    client.shutdown();
}

/// A hired rental administrator raises the flood threshold and prints the super-administrator warning.
#[tokio::test]
async fn should_match_intrinsics_for_a_super_admin_flood() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;

    client
        .setup_blockchain(&format!("Sa{suffix}"), &format!("w{suffix}"))
        .await
        .unwrap();

    let network = ssh_network_of_kind(&client, "RentServer").await;
    let account = fixture_account(&network, "ssh", "ssh", false);

    join_fixture(&client, &network).await;
    client.hire_rental_admin(&network.public_ip).await.unwrap();
    client.prime_flood(&account.address).await.unwrap();

    let snapshot = client.network_state().await.unwrap();

    assert!(snapshot.rental_admin, "super-admin-flood");

    let threshold = u32::from(snapshot.net_speed) * 3;
    let floods = (1..threshold)
        .map(|origin| {
            (
                account.address.clone(),
                format!("fixture-{origin}"),
                threshold,
            )
        })
        .collect::<Vec<_>>();
    let scripts = vec![format!(
        "mx = include_lib(\"/lib/metaxploit.so\"); session = mx.net_use(\"{}\", {}); print session.flood_connection",
        account.address, account.port
    )];

    compare_in_flooded_labeled(
        &client,
        &user,
        &snapshot,
        "super-admin-flood",
        &scripts,
        &floods,
    )
    .await;
    client.shutdown();
}

/// A flood on a listed company reaches its reboot and records the DDoS ranking penalty.
#[tokio::test]
async fn should_match_intrinsics_for_a_listed_company_flood() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let suffix = std::process::id() % 10000;
    let bank = client
        .setup_blockchain(&format!("Lc{suffix}"), &format!("w{suffix}"))
        .await
        .unwrap()
        .bank_number
        .clone();
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);

    join_fixture(&client, &network).await;
    client
        .setup_guild(GuildSetup {
            name: format!("Listed{suffix}"),
            bank_account: bank,
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            join: true,
            company_ip: network.public_ip.clone(),
            crown: false,
            shield_ip: String::new(),
            admin_restore_perks: 0,
            admin_restore_off: false,
        })
        .await
        .unwrap();
    client.prime_flood(&account.address).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let super_admin = snapshot.rental_admin
        || snapshot.admin.as_ref().is_some_and(|admin| {
            admin.work_performance > 2
                && u16::midpoint(
                    u16::from(admin.intelligence),
                    u16::from(admin.pc_experience),
                ) >= 8
                && !admin.dead_or_fired
        });
    let mut threshold = u32::from(snapshot.net_speed) * 3;

    if !super_admin {
        threshold = (threshold * 3).div_ceil(4);
    }

    let floods = (1..threshold)
        .map(|origin| {
            (
                account.address.clone(),
                format!("fixture-{origin}"),
                threshold,
            )
        })
        .collect::<Vec<_>>();
    let scripts = vec![format!(
        "mx = include_lib(\"/lib/metaxploit.so\"); session = mx.net_use(\"{}\", {}); print session.flood_connection",
        account.address, account.port
    )];

    compare_in_flooded_labeled(
        &client,
        &user,
        &snapshot,
        "listed-company-flood",
        &scripts,
        &floods,
    )
    .await;

    let companies = client
        .query_guilds(std::slice::from_ref(&snapshot.public_ip))
        .await
        .unwrap();

    assert!(companies.companies[0].ddos_penalty);
    client.shutdown();
}

/// DNS lookups and the game clock mirrored at the snapshot (`PlayerIntrinsics`).
#[tokio::test]
async fn should_match_intrinsics_for_nslookup_and_the_current_date() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let shop = empty_network(&client, "HackShop").await;
    let snapshot = join_fixture(&client, &shop).await;
    let scripts = vec![
        format!(
            "print current_date; print nslookup(\"{}\"); print nslookup(\"!!!\"); print nslookup(\"missing.invalid\")",
            snapshot.domain
        ),
        "print nslookup(\"\")".to_owned(),
        "print nslookup(1)".to_owned(),
    ];

    let outcomes = compare_in(&client, &user, &snapshot, &scripts).await;

    assert!(
        outcomes[0]
            .output
            .as_deref()
            .is_some_and(|output| output.contains(&shop.public_ip.clone()))
    );
    client.shutdown();
}

/// `PlayerUtils.PreRenameFichero` saves immediately. `Router.GetPortVersion` therefore loses the service library as
/// soon as it is renamed, and a later script still sees `unknown` rather than its former version.
#[tokio::test]
async fn should_match_intrinsics_for_port_info_after_a_library_rename() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);
    let port = account.port;
    let root_password = account.root_password;
    let snapshot = join_fixture(&client, &network).await;
    // `RouterIntrinsics.ping_port` is the router's way to a port object by the number it forwards from; `get_port`
    // does not exist.
    let rename = format!(
        r#"remote = get_shell.connect_service("{}", {port}, "root", {})
library = remote.host_computer.File("/lib/libssh.so")
router = get_router
print router.port_info(router.ping_port({port}))
print library.rename("renamed.so")
print router.port_info(router.ping_port({port}))"#,
        snapshot.public_ip,
        script_string(&root_password),
    );

    let before = compare_in(&client, &user, &snapshot, &[rename]).await;

    assert_succeeded(&before);
    assert_port_info_lines(before[0].output.as_deref());

    let renamed = client.network_state().await.unwrap();
    let later = format!(
        r#"router = get_router
print router.port_info(router.ping_port({port}))"#
    );
    let after = compare_in(&client, &user, &renamed, &[later]).await;

    assert_succeeded(&after);
    assert_port_info_lines(after[0].output.as_deref());
    client.shutdown();
}
