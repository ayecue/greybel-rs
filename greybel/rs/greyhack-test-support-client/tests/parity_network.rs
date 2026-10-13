//! Networks: the player's network, reaching addresses and services, routers, ports, DNS, FTP and network sessions.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{join_network, join_network_where, setup_player, test_client};
use crate::support::metaxploit::{overflow_script, router_library, weakness};
use crate::support::parity::{
    assert_port_info_lines, assert_succeeded, compare_in, compare_in_flooded, game_outcomes,
    intrinsics_outcomes, snapshot_outcomes,
};
use crate::support::remote::{
    forwarded_root, forwarded_service, script_string, service_account, service_behind_devices,
};

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

/// The player's network, the simulated one laid out like the game's: its main router, LAN router and switch, what
/// they list and forward, what answers a ping and what a connection meets on the way.
#[tokio::test]
async fn should_match_intrinsics_for_the_players_network() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = join_network_where(&client, "ssh", |snapshot| {
        let router = |kind: &str| {
            snapshot.nodes.iter().any(|node| {
                node.parent.is_some()
                    && matches!(&node.node, LanNodeKind::Router(router) if router.kind == kind)
            })
        };

        router("ROUTER") && router("SWITCH")
    })
    .await;
    let child_router = snapshot
        .nodes
        .iter()
        .find(|node| {
            node.parent.is_some()
                && matches!(&node.node, LanNodeKind::Router(router) if router.kind == "ROUTER")
        })
        .expect("the network has a LAN router")
        .local_ip
        .to_string();
    let switch = snapshot
        .nodes
        .iter()
        .find(|node| matches!(&node.node, LanNodeKind::Router(router) if router.kind == "SWITCH"))
        .expect("the network has a switch")
        .local_ip
        .to_string();
    let computer = snapshot
        .nodes
        .iter()
        .find(|node| matches!(node.node, LanNodeKind::Computer(_)))
        .expect("the network has a computer")
        .local_ip
        .to_string();
    let mut addresses: Vec<String> = snapshot
        .nodes
        .iter()
        .filter(|node| !node.local_ip.is_empty())
        .map(|node| node.local_ip.to_string())
        .collect();

    addresses.push("10.0.0.1".to_owned());

    let forwarded_port = match &snapshot.nodes[0].node {
        LanNodeKind::Router(router) => {
            router
                .ports
                .iter()
                .find(|port| !port.local_ip.is_empty())
                .expect("the main router forwards a service")
                .number
        }
        _ => panic!("the first node is the main router"),
    };
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
                .map(|port| (node.local_ip.to_string(), port.number))
        })
        .collect::<Vec<_>>();
    let mut scripts = vec![
        "r = get_router; print [r.public_ip, r.local_ip, r.essid_name, r.bssid_name, r.kernel_version]; print r.devices_lan_ip; print r.firewall_rules".to_owned(),
        format!("r = get_router; for p in r.used_ports; print [p.port_number, p.is_closed, p.get_lan_ip, r.port_info(p)]; end for; print r.ping_port({forwarded_port}).get_lan_ip; print r.ping_port(8080)"),
        format!("r = get_router(\"{child_router}\"); print [r.local_ip, r.public_ip, r.essid_name, r.kernel_version, r.used_ports.len]; print r.devices_lan_ip; print r.firewall_rules"),
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
    let service = service_behind_devices(&client, &["ssh"]).await;
    let host = service.address;
    let port = service.port;
    let password = service.root_password;
    let devices = service.devices;

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
    let police = client.find_network("Comisaria", "").await.unwrap();

    join_network(&client, &police.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let account = service_account(&snapshot, "ftp");
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

/// SSH and FTP run a reached computer's encode/decode scripts for successful and failed logins, retry a changed
/// decoded password, and honour the encryption message (`ShellIntrinsics.connect_service`, `PlayerUtils.ConnectService`).
#[tokio::test]
async fn should_match_intrinsics_for_encrypted_service_logins() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let (address, snapshot, port, password, ftp_port, ftp_password) = loop {
        let network = client
            .find_new_network("Supermercados", "ssh")
            .await
            .unwrap();
        let address = network.public_ip.to_string();

        join_network(&client, &address).await;

        let snapshot = client.network_state().await.unwrap();

        if forwarded_service(&snapshot, "ftp").is_none() {
            continue;
        }

        let (port, password) = forwarded_root(&snapshot, "ssh");
        let (ftp_port, ftp_password) = forwarded_root(&snapshot, "ftp");

        break (address, snapshot, port, password, ftp_port, ftp_password);
    };
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

/// A router's own file system, reached as guest through its kernel's computer reward: the router layout, its website
/// and its kernel (`XmlGlobal.GenerarRouterFileSystem`, `MetaLibIntrinsics.overflow`, `Router.GetComputer`,
/// `RouterIntrinsics.kernel_version`).
#[tokio::test]
async fn should_match_intrinsics_for_a_routers_file_system() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let mut found = None;

    // A fresh network whose kernel has a computer reward without requirements.
    for _ in 0..8 {
        let network = client
            .find_new_network("Supermercados", "ssh")
            .await
            .unwrap();
        let address = network.public_ip.to_string();

        join_network(&client, &address).await;

        let snapshot = client.network_state().await.unwrap();
        let main = snapshot
            .nodes
            .iter()
            .find(|node| node.parent.is_none())
            .map(|node| node.local_ip.to_string())
            .expect("the network has a main router");
        let kernel = router_library(&snapshot, &main);
        let usable = weakness(&kernel, |weakness| {
            weakness.reward == Reward::Computer
                && weakness.requirements.is_empty()
                && weakness.user == "guest"
        })
        .map(|(zone, weakness)| (zone.to_owned(), weakness.clone()));

        if let Some((zone, weakness)) = usable {
            found = Some((address, main, snapshot, zone, weakness));
            break;
        }
    }

    let (address, main, snapshot, zone, weakness) =
        found.expect("a fresh network's kernel has a computer reward without requirements");
    let routers: Vec<String> = snapshot
        .nodes
        .iter()
        .filter(|node| matches!(node.node, LanNodeKind::Router(_)))
        .map(|node| node.local_ip.to_string())
        .collect();
    let scripts: Vec<String> = routers
        .iter()
        .map(|router| {
            let exploit = overflow_script("", Some((&main, 0)), &zone, &weakness, router);

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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();
    let address = network.public_ip.to_string();

    join_network(&client, &address).await;

    let snapshot = client.network_state().await.unwrap();
    let (port, password) = forwarded_root(&snapshot, "ssh");
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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();
    let address = network.public_ip.to_string();

    join_network(&client, &address).await;
    client.disable_network(&address, true).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let (port, password) = forwarded_root(&snapshot, "ssh");
    let (lan_ip, lan_port, lan_password) = snapshot
        .nodes
        .iter()
        .find_map(|node| {
            if node.hidden {
                return None;
            }

            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };
            let port = computer
                .ports
                .iter()
                .find(|port| port.service == "ssh" && !port.closed)?;
            let root = computer.users.iter().find(|user| user.name == "root")?;

            Some((
                node.local_ip.to_string(),
                port.number,
                root.password.to_string(),
            ))
        })
        .expect("a visible LAN computer running SSH");
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
    let mut administered = None;

    for _ in 0..32 {
        let network = client.spawn_network("Supermercados").await.unwrap();

        join_network(&client, &network.public_ip).await;

        let snapshot = client.network_state().await.unwrap();
        let ssh = snapshot.nodes.iter().any(|node| {
            let LanNodeKind::Computer(computer) = &node.node else {
                return false;
            };

            computer.users.iter().any(|user| user.name == "root")
                && computer.users.iter().any(|user| user.name != "root")
                && computer
                    .ports
                    .iter()
                    .any(|port| port.service == "ssh" && !port.closed)
        });

        if snapshot.admin.is_some() && ssh {
            administered = Some(snapshot);
            break;
        }
    }

    let before = administered.expect("a generated supermarket has an administrator and SSH");
    let account = service_account(&before, "ssh");
    let main = before.nodes[0].local_ip.to_string();

    client.prime_flood(&account.address).await.unwrap();
    client.prime_flood(&main).await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let account = service_account(&snapshot, "ssh");
    let super_admin = snapshot.rental_admin
        || snapshot.admin.as_ref().is_some_and(|admin| {
            admin.work_performance > 2
                && (u16::from(admin.intelligence) + u16::from(admin.pc_experience)) / 2 >= 8
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

/// DNS lookups and the game clock mirrored at the snapshot (`PlayerIntrinsics`).
#[tokio::test]
async fn should_match_intrinsics_for_nslookup_and_the_current_date() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let shop = client.find_network("HackShop", "").await.unwrap();

    join_network(&client, &shop.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
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
            .is_some_and(|output| output.contains(&shop.public_ip.to_string()))
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
    // The forward may point at a computer the outside can't log in to (a firewall on the way, as
    // `PlayerUtils.ConnectComputer` checks), so the network is one where the simulated copy logs in; the comparison
    // below still runs the same script in the game.
    let mut found = None;

    for _ in 0..12 {
        let network = client
            .find_new_network("Supermercados", "ssh")
            .await
            .unwrap();

        join_network(&client, &network.public_ip).await;

        let snapshot = client.network_state().await.unwrap();
        let (port, root_password) = forwarded_root(&snapshot, "ssh");
        let login = format!(
            "print typeof(get_shell.connect_service(\"{}\", {port}, \"root\", {}))",
            snapshot.public_ip,
            script_string(&root_password),
        );

        let outcome = &snapshot_outcomes(&[&login], &user, &snapshot)[0];

        if outcome.output.as_deref().map(str::trim_end) == Some("shell") {
            found = Some((snapshot, port, root_password));
            break;
        }
    }

    let (snapshot, port, root_password) =
        found.expect("a generated network whose forwarded SSH computer takes root logins");
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
