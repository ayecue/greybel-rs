//! The player's computer: files, users and groups, addresses, reboots, startup files and services.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{
    dual_service_network, join_fixture, join_network, mail_network, router_ssh_network,
    setup_player, smartphone_network, smtp_network, ssh_network, test_client,
};
use crate::support::metaxploit::{
    computer_library, fixture_reply_weakness, fixture_weakness, overflow_script,
};
use crate::support::parity::{compare_in, game_outcomes, intrinsics_outcomes};
use crate::support::parity_computers::{ethernet_addresses, replace_last_octet};
use crate::support::remote::{fixture_account, script_string};

/// The `File` class on files the script makes itself. Creation and lookup of a file are in separate scripts: the game
/// updates what a script sees of a new entry some time after the call returns. A copy or move isn't read back at once
/// for the same reason.
#[tokio::test]
async fn should_match_intrinsics_for_file_operations() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    // Each probe has its own folder: the game may still show the previous script's folder, deleted at its end, for
    // a moment after that script finished, so a shared name would let one script see another's leftovers.
    let setup = |index: usize| {
        format!(
            "pc = get_shell.host_computer; pc.create_folder(home_dir, \"probe{index}\"); d = pc.File(home_dir + \"/probe{index}\"); pc.touch(d.path, \"a.txt\")"
        )
    };
    let prelude = |index: usize| {
        format!(
            "pc = get_shell.host_computer; d = pc.File(home_dir + \"/probe{index}\"); f = pc.File(d.path + \"/a.txt\")"
        )
    };
    let operations = [
        "print [d.name, d.path, d.is_folder, d.is_binary, d.is_symlink, d.size, d.permissions, d.owner, d.group, d.parent.path, d.has_permission(\"r\"), d.has_permission(\"x\"), d.has_permission(\"q\")]; print [f.name, f.size, f.permissions, f.owner, f.group, f.get_content, f.is_folder, f.allow_import, f.get_files, f.get_folders]; print f.set_content(\"hello\"); print [f.size, f.get_content]; print d.get_files.len; print d.get_folders; print typeof(d.get_files[0])",
        "print f.chmod(\"u-w\"); print f.permissions; print f.chmod(\"o+rwx\"); print f.permissions; print f.chmod(\"a-x\"); print f.permissions; print f.chmod(\"bad\"); print f.chmod(\"u+w\", 1); print f.chmod(\"g=r\"); print f.chmod(\"777\"); print f.permissions",
        "print f.copy(d.path, \"b.txt\"); f = pc.File(f.path); print f.copy(\"/nope\", \"c.txt\"); print f.copy(d.path, \"\"); print f.rename(\"\"); print f.rename(\"a/b\")",
        "print f.move(d.path, \"m.txt\")",
        "print f.rename(\"r.txt\"); print f.path",
        "print f.set_owner(\"root\"); print f.owner; print f.set_owner(\"nobody\"); print f.set_group(\"root\"); print f.group; print f.set_group(\"nogroup\"); print d.set_owner(\"root\", 1); print d.set_group(\"guest\", 1); print f.delete; print f.delete",
        "print pc.touch(d.path, \"a.txt\"); print pc.touch(\"/nope\", \"x\"); print pc.touch(d.path, \"\"); print pc.touch(d.path, \"bad name!\"); print pc.create_folder(d.path, \"sub\"); print pc.create_folder(d.path, \"sub\"); print pc.create_folder(\"/nope\", \"x\"); print pc.create_folder(d.path, \"\"); print pc.File(\"/nope\"); print pc.File(\"relative\"); print pc.File(\"\")",
        "print pc.File(\"/bin/ls\").get_content; print pc.File(\"/bin/ls\").set_content(\"x\"); print pc.File(\"/bin/ls\").allow_import; print pc.File(\"/etc/passwd\").permissions; print pc.File(\"/\").name; print pc.File(\"/\").parent; print pc.File(\"/\").permissions; print pc.File(\"/root\").permissions; print pc.File(home_dir).permissions; print pc.File(home_dir + \"/Config\").permissions; print pc.File(\"/home/guest/Config\").permissions",
    ];
    let mut scripts = Vec::new();

    for (index, script) in operations.iter().enumerate() {
        scripts.push(setup(index));
        scripts.push(format!("{}\n{script}\nd.delete", prelude(index)));
    }

    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = intrinsics_outcomes(&scripts, &user);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// The `Computer` class and the player's globals, on what every new player's computer has.
#[tokio::test]
async fn should_match_intrinsics_for_the_player_computer() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts: Vec<String> = [
        "pc = get_shell.host_computer; print [typeof(pc), is_valid_ip(pc.local_ip), is_valid_ip(pc.public_ip), is_lan_ip(pc.local_ip), is_lan_ip(pc.public_ip), pc.is_network_active, typeof(pc.network_gateway), typeof(pc.get_ports), typeof(pc.show_procs), typeof(pc.network_devices), pc.active_net_card]; print pc.wifi_networks(\"bad\"); print pc.groups(\"test\"); print pc.groups(\"root\"); print pc.groups(\"nobody\")",
        "pc = get_shell.host_computer; print pc.create_user(\"probeu\", \"pw\"); print pc.create_user(\"bad name\", \"pw\"); print pc.create_user(\"\", \"pw\")",
        "pc = get_shell.host_computer; print pc.show_procs.split(char(10))[0]; print pc.close_program(999999); print pc.close_program(-1); print pc.connect_wifi(\"wlan0\", \"00:00:00:00:00:00\", \"x\", \"y\"); print pc.connect_ethernet(\"eth0\", \"10.0.0.5\", \"10.0.0.1\"); print pc.connect_ethernet(\"bad\", \"x\", \"y\")",
        "print format_columns(\"NAME VALUE\" + char(10) + \"A 42\" + char(10) + \"longer_name 1\"); print format_columns(\"\"); print format_columns(\"a b c\" + char(10) + \"dd ee\" + char(10) + \"f\"); print format_columns(\"<b>A</b> B\" + char(10) + \"C D\")",
        "print current_path; print home_dir; print program_path; print launch_path; print parent_path(\"/a/b/c\"); print parent_path(\"/\"); print parent_path(\"a/b\"); print get_abs_path(\"x\"); print get_abs_path(\"/x/../y\"); print get_abs_path(\"../z\"); print active_user; print user_mail_address; print user_bank_number",
        "print is_valid_ip(\"1.2.3.4\"); print is_valid_ip(\"1.2.3\"); print is_valid_ip(\"256.1.1.1\"); print is_lan_ip(\"10.0.0.1\"); print is_lan_ip(\"172.16.0.1\"); print is_lan_ip(\"8.8.8.8\")",
        "cd(\"/bin\"); print current_path; cd(\"nope\"); print current_path; cd(\"..\"); print current_path; cd(\"/etc/passwd\"); print current_path",
        "print command_info(\"LS_USAGE\"); print command_info(\"nope\"); print command_info(\"\"); print command_info(\"LS\")",
        "print get_router(1); print get_switch(1); print get_shell(1, \"x\"); print get_shell(\"root\", \"wrong\"); print cd(1); print mail_login(1, \"x\")",
        "print get_abs_path(1); print get_abs_path(\"x\", 1); print get_abs_path(\"x\" * 1025); print get_abs_path(\"x\", \"b\" * 65)",
        "print user_input(1); print user_input(\"\", 1, 1, 1, \"x\"); print include_lib(\"\"); print include_lib(1)",
        "print whois(\"\"); print whois(1); print whois(\"bad\"); print whois(\"10.0.0.1\"); print parent_path(\"\"); print parent_path(1); print format_columns(1)",
        "print is_valid_ip(\"\"); print is_valid_ip(1); print is_lan_ip(\"\"); print is_lan_ip(1)",
        "pc = get_shell.host_computer; cd(home_dir); pc.File(home_dir).delete; cd(\"/bin\"); print current_path",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let snapshot = client.network_state().await.unwrap();

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// Configured player identity values, their remote-host nulls, and offline mail login.
#[tokio::test]
async fn should_match_intrinsics_for_configured_player_globals_and_offline_mail() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let mail = smtp_network(&client).await;

    join_fixture(&client, &mail).await;

    client.setup_mail("parity", "mailpass1").await.unwrap();

    let suffix = std::process::id() % 10000;

    client
        .setup_blockchain(&format!("PlayerBank{suffix}"), &format!("wallet{suffix}"))
        .await
        .unwrap();

    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", true);

    join_fixture(&client, &network).await;

    let snapshot = client.network_state().await.unwrap();
    let identity = script_string("print user_mail_address; print user_bank_number");
    let script = format!(
        "print user_mail_address; print user_bank_number; remote = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); pc = remote.host_computer; pc.touch(\"/root\", \"identity.src\"); pc.File(\"/root/identity.src\").set_content({identity}); remote.build(\"/root/identity.src\", \"/root\"); remote.launch(\"/root/identity\"); get_shell.host_computer.File(\"/lib/init.so\").delete; print get_router; print mail_login(\"a@b\", \"x\")",
        account.address, account.port, account.root_password
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// A shell on an NPC workstation reports that workstation's disabled account, not a bad password.
#[tokio::test]
async fn should_match_intrinsics_for_a_disabled_shell_account() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = mail_network(&client, 1).await;
    let snapshot = join_fixture(&client, &network).await;
    let npc = snapshot
        .npcs
        .iter()
        .find(|npc| {
            npc.mail.is_some()
                && snapshot
                    .admin
                    .as_ref()
                    .is_none_or(|admin| admin.user != npc.user)
        })
        .expect("an NPC account on an SSH workstation");
    let address = npc.local_ip.clone();
    let fired = npc.user.clone();
    let password = npc.mail.as_ref().unwrap().password.clone();

    client.fire_npc(&network.public_ip, &fired).await.unwrap();
    client
        .refresh_npc_account(&network.public_ip, &fired)
        .await
        .unwrap();
    client
        .configure_computer(&address, "ssh_service")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let root_password = snapshot
        .nodes
        .iter()
        .find_map(|node| {
            if node.local_ip != address {
                return None;
            }

            let LanNodeKind::Computer(computer) = &node.node else {
                return None;
            };

            computer
                .users
                .iter()
                .find(|user| user.name == "root")
                .map(|user| user.password.clone())
        })
        .expect("the workstation root password");
    let login = script_string(&format!("print get_shell(\"{fired}\", \"{password}\")"));
    let script = format!(
        "remote = get_shell.connect_service(\"{}\", 11022, \"root\", \"{}\"); pc = remote.host_computer; pc.touch(\"/root\", \"login.src\"); pc.File(\"/root/login.src\").set_content({login}); remote.build(\"/root/login.src\", \"/root\"); remote.launch(\"/root/login\")",
        network.public_ip, root_password
    );

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// Service objects included by library content and malformed library JSON.
#[tokio::test]
async fn should_match_intrinsics_for_service_includes_and_malformed_libraries() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    for name in ["libftp.so", "librepository.so"] {
        client
            .install_library("", name, u8::try_from(Reward::Shell.code()).unwrap())
            .await
            .unwrap();
    }

    client
        .configure_computer("", "malformed_metaxploit")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let scripts = [
        "print typeof(include_lib(\"/lib/libftp.so\")); print typeof(include_lib(\"/lib/librepository.so\")); print include_lib(\"/etc/passwd\"); print include_lib(\"/lib/missing.so\")",
        "include_lib(\"/lib/metaxploit.so\")",
    ]
    .map(str::to_owned);

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// A computer whose service library isn't the library its name says refuses the service, and one missing or with a
/// corrupted startup file doesn't start (`Library.IsValidLibrary`, `Computer.CheckInicio`).
#[tokio::test]
async fn should_match_intrinsics_for_libraries_and_startup_files() {
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
    let libraries = format!(
        "s = {connect}; print typeof(s); pc = s.host_computer; mx = include_lib(\"/lib/metaxploit.so\"); \
         print pc.File(\"/lib/libssh.so\").rename(\"libssh_old.so\"); print get_shell.scp(\"/lib/crypto.so\", \"/root\", s); \
         print pc.File(\"/root/crypto.so\").move(\"/lib\", \"libssh.so\"); print {connect}; print mx.net_use(\"{address}\", {port}); \
         print pc.File(\"/lib/libssh.so\").delete; print pc.File(\"/lib/libssh_old.so\").rename(\"libssh.so\"); \
         print typeof({connect})"
    );

    compare_in(&client, &user, &snapshot, &[libraries]).await;

    let replaced = client.network_state().await.unwrap();
    let startup = format!(
        "s = {connect}; pc = s.host_computer; mx = include_lib(\"/lib/metaxploit.so\"); \
         print pc.File(\"/boot/System.map\").delete; print pc.File(\"/lib/net.so\").delete; \
         print get_shell.scp(\"/lib/crypto.so\", \"/root\", s); \
         print pc.File(\"/root/crypto.so\").move(\"/lib\", \"net.so\"); print pc.is_network_active; \
         print typeof(mx.net_use(\"{address}\", {port}))"
    );

    compare_in(&client, &user, &replaced, &[startup]).await;

    // Another program reaches the computer as the game saved it, without the startup files.
    let broken = client.network_state().await.unwrap();
    let reconnect = format!("print {connect}");

    compare_in(&client, &user, &broken, &[reconnect]).await;
    client.shutdown();
}

/// Account and group management as root and as a user without those privileges (`ComputerIntrinsics`).
#[tokio::test]
async fn should_match_intrinsics_for_computer_account_management() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = [
        r#"pc = get_shell.host_computer
print pc.create_user("managua", "oldpass1")
print pc.change_password("managua", "newpass1")
print typeof(get_shell("managua", "newpass1"))
print pc.change_password("missing", "valid1")
print pc.change_password("managua", "bad pass")
print pc.change_password("managua", "1234567890123456")
other = get_shell("managua", "newpass1").host_computer
print other.change_password("managua", "other1")
print other.create_group("managua", "team")
print other.delete_group("managua", "team")
print other.delete_user("managua")
print pc.delete_user("managua")"#
            .to_owned(),
        r#"pc = get_shell.host_computer
print pc.create_user("groupua", "pass1")
print pc.create_group("groupua", "team")
print pc.create_group("groupua", "team")
print pc.groups("groupua")
print pc.create_group("groupua", "bad name")
print pc.create_group("missing", "team")
print pc.delete_group("groupua", "missing")
print pc.delete_group("groupua", "team")
print pc.delete_group("groupua", "team")
print pc.delete_group("missing", "team")
print pc.delete_user("groupua")"#
            .to_owned(),
        r#"pc = get_shell.host_computer
print pc.create_user("delhome", "pass1")
print pc.File("/home/delhome") != null
print pc.delete_user("delhome", 1)
print pc.File("/home/delhome")
print pc.delete_user("delhome")
print pc.delete_user("root")
print pc.delete_user("test")
print pc.delete_user(1)"#
            .to_owned(),
        r#"pc = get_shell.host_computer; print pc.change_password("", "valid1")"#.to_owned(),
        r#"pc = get_shell.host_computer; print pc.change_password(1, "valid1")"#.to_owned(),
        r#"pc = get_shell.host_computer; print pc.create_group("", "team")"#.to_owned(),
        r#"pc = get_shell.host_computer; print pc.create_group("test", "1234567890123456")"#
            .to_owned(),
        r#"pc = get_shell.host_computer; print pc.delete_group("test", "")"#.to_owned(),
        r#"pc = get_shell.host_computer; print pc.delete_user("")"#.to_owned(),
        r#"pc = get_shell.host_computer; print pc.delete_user("test", "no")"#.to_owned(),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// Public addresses and the reboot checks a script can complete on a remote computer. A local reboot pauses the VM
/// instead of returning, so only its argument validation is compared (`ComputerIntrinsics.reboot`).
#[tokio::test]
async fn should_match_intrinsics_for_public_ips_and_a_remote_reboot() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = ssh_network(&client).await;
    let account = fixture_account(&network, "ssh", "ssh", false);

    let snapshot = join_fixture(&client, &network).await;
    let address = network.public_ip.clone();
    let scripts = vec![
        format!(
            "print get_shell.host_computer.public_ip; remote = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); print remote.host_computer.public_ip; print get_shell.host_computer.reboot(\"bad\")",
            account.address, account.port, account.root_password
        ),
        format!(
            "remote = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); pc = remote.host_computer; print pc.reboot(1); other = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\"); print other.host_computer.reboot; print pc.reboot",
            account.address,
            account.port,
            account.root_password,
            account.address,
            account.port,
            account.user,
            account.password
        ),
    ];

    let outcomes = compare_in(&client, &user, &snapshot, &scripts).await;

    assert_eq!(
        outcomes[0].output.as_deref(),
        Some(format!("{address}\n{address}\nnull").as_str())
    );
    client.shutdown();
}

/// File argument validation, stale handles, links, renames, and detached calls (`FileIntrinsics`).
#[tokio::test]
async fn should_match_intrinsics_for_file_edges() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let prelude = |index: usize| {
        format!(
            "pc = get_shell.host_computer; sh = get_shell; pc.create_folder(home_dir, \"edge{index}\"); d = pc.File(home_dir + \"/edge{index}\"); pc.touch(d.path, \"a.txt\"); f = pc.File(d.path + \"/a.txt\"); deep = \"/\" + \"a/\" * 17"
        )
    };
    let scripts = [
        "print f.copy(1, \"x\"); print f.copy(d.path, 1); print f.copy(d.path, \"bad name\"); print f.copy(d.path, \"x\" * 129); print f.copy(deep, \"x\")",
        "print copy({}, d.path, \"x\"); f.delete; print f.copy(d.path, \"x\")",
        "print f.move(1, \"x\"); print f.move(d.path, 1); print f.move(d.path, \"bad name\"); print f.move(d.path, \"x\" * 129); print f.move(deep, \"x\")",
        "print move({}, d.path, \"x\"); f.delete; print f.move(d.path, \"x\")",
        "print f.copy(\"bad\" + char(0) + \"path\", \"x\"); print f.move(\"bad\" + char(0) + \"path\", \"x\")",
        "print f.copy(d.path, \"a.txt\"); pc.create_folder(d.path, \"sub\"); print f.copy(d.path, \"sub\")",
        "print pc.File(\"/\").move(home_dir, \"root\"); print pc.File(\"/bin\").move(home_dir, \"bin\")",
        "print f.symlink(1, \"x\"); print f.symlink(d.path, 1); print f.symlink(d.path, \"bad name\"); print f.symlink(d.path, \"x\" * 129)",
        "print f.symlink(d.path, \"link\"); l = pc.File(d.path + \"/link\"); print [l.is_symlink, l.path, l.path(1), l.get_content]",
        "print d.symlink(home_dir, \"dirlink\"); l = pc.File(home_dir + \"/dirlink\"); print [l.is_symlink, l.is_folder, l.path, l.path(1)]",
        "print f.symlink(f.path, \"x\"); print f.symlink(\"/missing\", \"x\"); print f.symlink(d.path, \"a.txt\"); print f.symlink(deep, \"x\"); print f.symlink(\"bad\" + char(0) + \"path\", \"x\")",
        "print symlink({}, d.path, \"x\"); f.delete; print f.symlink(d.path, \"x\")",
        "print f.rename(null); print f.rename(\"x\" * 129); print rename({}, \"x\")",
        "f.delete; print f.rename(\"x\"); print pc.File(\"/\").rename(\"x\"); print pc.File(\"/bin\").rename(\"x\"); pc.touch(d.path, \"other\"); print f.rename(\"other\")",
        "l = pc.File(\"/var/system.log\"); print l.rename(\"system_old.log\"); print l.path; print pc.File(\"/var/system_old.log\").rename(\"system.log\")",
        "f.set_content(\"print 1\"); print sh.build(f.path, d.path, 1); print pc.File(d.path + \"/a\").allow_import; print allow_import({})",
        "print chmod({}, \"u+w\", 0); f.delete; print f.chmod(\"u+w\"); print pc.File(\"/bin/ls\").chmod(\"u+w\")",
        "print get_content({}); f.delete; print f.get_content; print d.get_content",
        "print f.set_content(\"x\" * 160001)",
        "print set_content({}, \"x\"); print f.set_content(null); f.delete; print f.set_content(\"x\"); print d.set_content(\"x\")",
        "print is_binary({}); print is_symlink({}); f.delete; print f.is_binary; print f.is_symlink",
        "print [permissions({}), owner({}), group({}), size({}), date({}), name({})]; f.delete; print [f.permissions, f.owner, f.group, f.size, f.date, f.name]",
        "print f.set_owner(\"bad name\"); print f.set_owner(\"x\" * 16); print f.set_owner(\"root\", \"x\"); print f.set_group(\"bad name\"); print f.set_group(\"x\" * 16); print f.set_group(\"root\", \"x\")",
        "print set_owner({}, \"root\", 0); print set_group({}, \"root\", 0); f.delete; print f.set_owner(\"root\"); print f.set_group(\"root\")",
        "print has_permission({}, \"r\"); f.delete; print f.has_permission(\"r\")",
        "print parent({}); print is_folder({}); f.delete; print f.parent; print f.is_folder",
        "print get_files({}); print get_folders({}); d.delete; print d.get_files; print d.get_folders",
    ]
    .iter()
    .enumerate()
    .map(|(index, script)| format!("{}\n{script}\nd.delete", prelude(index)))
    .collect::<Vec<_>>();
    let scripts: Vec<&str> = scripts.iter().map(String::as_str).collect();
    let expected = intrinsics_outcomes(&scripts, &user);
    let actual = game_outcomes(&client, &scripts).await;
    let differences: Vec<_> = actual
        .iter()
        .zip(&expected)
        .filter(|(actual, expected)| actual != expected)
        .collect();

    assert!(differences.is_empty(), "{differences:#?}");
    client.shutdown();
}

/// File operations as an unprivileged user on the bridge-built permission tree.
#[tokio::test]
async fn should_match_intrinsics_for_file_permissions() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client.configure_computer("", "permissions").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let prelude = "sh = get_shell(\"limited\", \"pass1\"); pc = sh.host_computer; root = get_shell(\"root\", \"test\").host_computer; b = \"/home/limited/matrix\"; f = pc.File(b + \"/open/readable.txt\"); locked = pc.File(b + \"/locked.txt\"); mixed = pc.File(b + \"/mixed\"); dest = b + \"/destination\"; denied = b + \"/destination_locked\"";
    let scripts = [
        "print locked.get_content; print locked.set_content(\"x\"); print [locked.has_permission(\"r\"), locked.has_permission(\"w\"), locked.has_permission(\"x\"), f.has_permission(\"w\")]",
        "d = pc.File(b + \"/locked\"); print d.get_files; print d.get_folders; print get_files({}); print get_folders({})",
        "print locked.copy(dest, \"copy.txt\"); print mixed.copy(dest, \"partial\")",
        "print f.copy(denied, \"copy.txt\"); print f.move(denied, \"move.txt\"); print locked.move(dest, \"move.txt\"); print locked.move(b, \"locked.txt\")",
        "print mixed.delete; print pc.File(b + \"/mixed\").get_files.len",
        "print locked.chmod(\"u+w\"); print f.set_owner(\"root\"); print f.set_group(\"root\")",
        "print f.symlink(denied, \"link\"); print pc.File(b + \"/mixed/locked/child.txt\").rename(\"other.txt\")",
        "print mixed.copy(b + \"/replace\", \"mixed\")",
    ]
    .map(|script| format!("{prelude}\n{script}"));

    compare_in(&client, &user, &snapshot, &scripts).await;

    client.shutdown();
}

/// Copy and create operations at the game's folder, filesystem, and disk boundaries.
#[tokio::test]
async fn should_match_intrinsics_for_file_resource_limits() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for (fixture, script) in [
        (
            "full_folder",
            "pc = get_shell.host_computer; d = home_dir + \"/full_folder\"; print pc.touch(d, \"new\"); print pc.create_folder(d, \"new\"); print pc.File(\"/etc/passwd\").copy(d, \"passwd\")",
        ),
        (
            "full_filesystem",
            "pc = get_shell.host_computer; d = home_dir + \"/full_filesystem\"; print pc.touch(d, \"new\"); print pc.create_folder(d, \"new\"); print pc.File(\"/etc/passwd\").copy(home_dir, \"boundary\")",
        ),
        (
            "full_disk",
            "pc = get_shell.host_computer; print pc.touch(home_dir, \"new\"); print pc.File(\"/etc/passwd\").copy(home_dir, \"boundary\"); print pc.File(\"/etc/passwd\").symlink(home_dir, \"link\")",
        ),
    ] {
        let user = setup_player(&client).await;

        client.configure_computer("", fixture).await.unwrap();

        let snapshot = client.network_state().await.unwrap();

        compare_in(&client, &user, &snapshot, &[script.to_owned()]).await;
    }

    client.shutdown();
}

/// Wi-Fi discovery and connection checks on ordinary, disabled, CTF, and smartphone computers.
#[tokio::test]
async fn should_match_intrinsics_for_wifi_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "pc = get_shell.host_computer; print wifi_networks({}, \"wlan0\"); print pc.wifi_networks(\"abc\"); print pc.wifi_networks(\"eth0\"); print pc.wifi_networks(1); x = pc.wifi_networks(\"wlan0\"); print typeof(x); print x.len > 0; print x[0].split(\" \").len == 3",
        "pc = get_shell.host_computer; print pc.connect_wifi(1, \"b\", \"e\", \"p\"); print pc.connect_wifi(\"bad\", \"b\", \"e\", \"p\"); print pc.connect_wifi(\"eth0\", \"b\", \"e\", \"p\")",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);
    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);

    let network = router_ssh_network(&client, true).await;

    join_fixture(&client, &network).await;

    let account = fixture_account(&network, "ssh", "ssh", true);
    let local_address = fixture_account(&network, "ssh", "ssh", false).address;
    let installed = client
        .install_fixture_library(
            &local_address,
            "libssh.so",
            vec![fixture_weakness(Reward::Shell, &[], "guest", "")],
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let library = computer_library(&snapshot, &local_address, "libssh.so");
    let (address, weakness) = fixture_reply_weakness(&library, &installed.weaknesses[0]);
    let guest = format!(
        "{}; print result.host_computer.connect_wifi(\"wlan0\", \"b\", \"e\", \"p\")",
        overflow_script(
            "",
            Some((&account.address, account.port)),
            address,
            weakness,
            "",
        )
    );
    let outcomes = compare_in(&client, &user, &snapshot, &[guest]).await;

    assert!(outcomes[0].output.as_deref().is_some_and(|output| {
        output.ends_with(
            "shell\nconnect_wifi: permission denied. Guest users can not execute this method",
        )
    }));

    let child = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Router(router) if node.parent.is_some() && router.bssid.is_some() => {
                Some(router)
            }
            _ => None,
        })
        .expect("a child Wi-Fi router");
    let bssid = child.bssid.as_deref().unwrap();
    let essid = child.essid.as_deref().unwrap();
    let password = &child.wifi_password;

    let scripts = vec![
        format!(
            "pc = get_shell.host_computer; print pc.connect_wifi(\"wlan0\", \"{bssid}\", \"{essid}\", \"wrong\")"
        ),
        format!(
            "pc = get_shell.host_computer; print pc.connect_wifi(\"wlan0\", \"{bssid}\", \"{essid}\", \"{password}\"); print pc.connect_wifi(\"wlan0\", \"{bssid}\", \"{essid}\", \"{password}\")"
        ),
    ];

    let outcomes = compare_in(&client, &user, &snapshot, &scripts).await;

    assert_eq!(
        outcomes[0].output.as_deref(),
        Some("Can't connect. Incorrect password.")
    );
    assert_eq!(
        outcomes[1].output.as_deref(),
        Some("1\nconnect_wifi: You must wait a moment before reconnecting to another network.")
    );

    let user = setup_player(&client).await;

    client
        .configure_computer("", "disabled_wifi")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let disabled_card =
        "pc = get_shell.host_computer; print pc.wifi_networks(\"wlan0\")".to_owned();

    compare_in(&client, &user, &snapshot, &[disabled_card]).await;

    let main = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Router(router) if node.parent.is_none() => Some(router),
            _ => None,
        })
        .expect("the main router");
    let bssid = main.bssid.as_deref().unwrap();
    let essid = main.essid.as_deref().unwrap();

    client
        .disable_network(&snapshot.public_ip, true)
        .await
        .unwrap();

    let disabled = client.network_state().await.unwrap();
    let script = format!(
        "pc = get_shell.host_computer; print pc.connect_wifi(\"wlan0\", \"{bssid}\", \"{essid}\", \"wrong\")"
    );

    compare_in(&client, &user, &disabled, &[script]).await;

    let user = setup_player(&client).await;
    let phone = smartphone_network(&client).await;
    let account = fixture_account(&phone, "phone", "ssh", true);
    let snapshot = join_fixture(&client, &phone).await;
    let script = format!(
        "shell = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); print shell.host_computer.connect_wifi(\"wlan0\", \"b\", \"e\", \"p\")",
        account.address, account.port, account.root_password
    );

    compare_in(&client, &user, &snapshot, &[script]).await;

    let ctf = client.spawn_ctf_network().await.unwrap();

    join_network(&client, &ctf.public_ip).await;

    client.freeze_clock().await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let script = "pc = get_shell.host_computer; x = pc.wifi_networks(\"wlan0\"); print typeof(x); good = 1; for item in x; if item.split(\" \").len != 3 then good = 0; end for; print good".to_owned();

    compare_in(&client, &user, &snapshot, &[script]).await;
    client.shutdown();
}

/// Computer detached calls, account management, process protection, and path validation.
#[tokio::test]
async fn should_match_intrinsics_for_computer_edges_and_accounts() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "print [get_ports({}), is_network_active(1), get_name({}), reboot({}, 0), active_net_card({}), network_gateway({}), show_procs({}), close_program({}, \"x\"), network_devices({})]",
        "pc = get_shell.host_computer; print pc.groups(\"\"); print pc.groups(1); print groups({}, \"root\"); print pc.create_group(1, \"x\"); print pc.delete_group(1, \"x\"); print create_group({}, \"test\", \"x\"); print delete_group({}, \"test\", \"x\")",
        "pc = get_shell.host_computer; print pc.create_user(1, \"pw\"); print pc.create_user(\"x\" * 16, \"pw\"); print pc.create_user(\"long\", \"x\" * 16); print pc.create_user(\"root\", \"pw\"); print pc.create_user(\"guest\", \"pw\"); print pc.create_user(\"existing\", \"pw\"); print pc.create_user(\"existing\", \"pw\")",
        "pc = get_shell.host_computer; print pc.touch(1, \"x\"); print pc.touch(home_dir, 1); print pc.touch(home_dir, \"x\" * 129); print pc.touch(\"/\" + \"a/\" * 17, \"x\"); print pc.touch(\"bad\" + char(0) + \"path\", \"x\"); print touch({}, home_dir, \"x\")",
        "pc = get_shell.host_computer; print pc.create_folder(1, \"x\"); print pc.create_folder(home_dir, 1); print pc.create_folder(home_dir, \"x\" * 129); print pc.create_folder(\"/\" + \"a/\" * 17, \"x\"); print pc.create_folder(\"bad\" + char(0) + \"path\", \"x\"); print pc.create_folder(\"\", \"x\"); print create_folder({}, home_dir, \"x\")",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);
    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);

    client.configure_computer("", "permissions").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let processes = [
        r#"pc = get_shell("limited", "pass1").host_computer
lines = pc.show_procs.split(char(10))
for line in lines
fields = line.split(" ")
if fields.len > 4 and fields[0] == "root" then
result = pc.close_program(fields[1].to_int)
print result.indexOf("belongs to user <b>root</b>") != null
break
end if
end for"#
            .to_owned(),
        r#"pc = get_shell("root", "test").host_computer
lines = pc.show_procs.split(char(10))
for line in lines
fields = line.split(" ")
if fields.len > 4 and fields[0] == "root" and fields[4] == "Xorg" then
print pc.close_program(fields[1].to_int)
break
end if
end for"#
            .to_owned(),
        "root = get_shell(\"root\", \"test\"); limited = get_shell(\"limited\", \"pass1\"); print root.host_computer.delete_user(\"limited\"); print limited.host_computer.File(\"/\")".to_owned(),
    ];

    compare_in(&client, &user, &snapshot, &processes).await;

    let user = setup_player(&client).await;

    client.configure_computer("", "users").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let full =
        "pc = get_shell.host_computer; print pc.create_user(\"overflow\", \"pw\")".to_owned();

    compare_in(&client, &user, &snapshot, &[full]).await;

    let user = setup_player(&client).await;

    client.configure_computer("", "deep_tree").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let deep = format!(
        "pc = get_shell.host_computer; print pc.create_folder(\"/{}\", \"new\")",
        vec!["d"; 16].join("/")
    );

    compare_in(&client, &user, &snapshot, &[deep]).await;
    client.shutdown();
}

/// Ethernet validation, local-network changes, and home-network fault handling.
#[tokio::test]
async fn should_match_intrinsics_for_ethernet_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let scripts = [
        "pc = get_shell.host_computer; print pc.connect_ethernet(1, \"10.0.0.5\", \"10.0.0.1\"); print connect_ethernet({}, \"eth0\", \"10.0.0.5\", \"10.0.0.1\"); print connect_ethernet({\"classID\": \"Computer\"}, \"eth0\", \"10.0.0.5\", \"10.0.0.1\"); print pc.connect_ethernet(\"bad\", \"10.0.0.5\", \"10.0.0.1\"); print pc.connect_ethernet(\"wlan0\", \"10.0.0.5\", \"10.0.0.1\")",
        "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"10.0.0.5\", \"bad\"); print pc.connect_ethernet(\"eth0\", \"10.0.0.5\", \"192.168.1.1\")",
    ];
    let expected = intrinsics_outcomes(&scripts, &user);
    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);

    let network = router_ssh_network(&client, true).await;

    join_fixture(&client, &network).await;

    let account = fixture_account(&network, "ssh", "ssh", true);
    let local_address = fixture_account(&network, "ssh", "ssh", false).address;
    let installed = client
        .install_fixture_library(
            &local_address,
            "libssh.so",
            vec![fixture_weakness(Reward::Shell, &[], "guest", "")],
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let library = computer_library(&snapshot, &local_address, "libssh.so");
    let (address, weakness) = fixture_reply_weakness(&library, &installed.weaknesses[0]);
    let guest = format!(
        "{}; print result.host_computer.connect_ethernet(\"eth0\", \"10.0.0.5\", \"10.0.0.1\")",
        overflow_script(
            "",
            Some((&account.address, account.port)),
            address,
            weakness,
            "",
        )
    );

    let outcomes = compare_in(&client, &user, &snapshot, &[guest]).await;

    assert!(outcomes[0].output.as_deref().is_some_and(|output| {
        output.ends_with(
            "shell\nconnect_ethernet: permission denied. Guest users can not execute this method",
        )
    }));

    let user = setup_player(&client).await;

    client
        .configure_computer("", "ethernet_home")
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let (gateway, player_ip) = ethernet_addresses(&snapshot);
    let unused = replace_last_octet(&gateway, "250");
    let second = replace_last_octet(&gateway, "251");
    let scripts = vec![
        "pc = get_shell.host_computer; print pc.wifi_networks(\"eth0\")".to_owned(),
        "pc = get_shell.host_computer; print pc.connect_wifi(\"eth0\", \"b\", \"e\", \"p\")"
            .to_owned(),
        format!(
            "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"{player_ip}\", \"{player_ip}\")"
        ),
        format!(
            "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"{unused}\", \"{second}\")"
        ),
        format!(
            "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"{gateway}\", \"{gateway}\")"
        ),
        format!("print get_router.connect_ethernet(\"eth0\", \"{unused}\", \"{gateway}\")"),
        format!(
            "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"{unused}\", \"{gateway}\"); print pc.connect_ethernet(\"eth0\", \"{second}\", \"{gateway}\")"
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;

    let user = setup_player(&client).await;

    client.configure_computer("", "missing_isp").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let (gateway, _) = ethernet_addresses(&snapshot);
    let unused = replace_last_octet(&gateway, "250");
    let missing_isp = format!(
        "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"{unused}\", \"{gateway}\")"
    );

    compare_in(&client, &user, &snapshot, &[missing_isp]).await;

    let user = setup_player(&client).await;

    client
        .configure_computer("", "ethernet_home")
        .await
        .unwrap();

    client
        .disable_network(&snapshot.public_ip, true)
        .await
        .unwrap();

    let disabled = client.network_state().await.unwrap();
    let script = format!(
        "pc = get_shell.host_computer; print pc.connect_ethernet(\"eth0\", \"{unused}\", \"{gateway}\")"
    );

    compare_in(&client, &user, &disabled, &[script]).await;

    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let offline = "pc = get_shell.host_computer; pc.File(\"/lib/init.so\").delete; print pc.connect_ethernet(\"eth0\", \"10.0.0.5\", \"10.0.0.1\")".to_owned();

    compare_in(&client, &user, &snapshot, &[offline]).await;

    let user = setup_player(&client).await;

    client.configure_computer("", "permissions").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let denied = "pc = get_shell(\"limited\", \"pass1\").host_computer; print pc.touch(\"/home/limited/matrix/destination_locked\", \"new\"); print pc.create_folder(\"/home/limited/matrix/destination_locked\", \"new\"); print cd(\"/home/limited/matrix/locked\"); print current_path".to_owned();

    compare_in(&client, &user, &snapshot, &[denied]).await;
    client.shutdown();
}

/// File and Computer intrinsics rejected while the server runs its SSH encoder.
#[tokio::test]
async fn should_match_intrinsics_for_file_and_computer_ssh_encryption() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = dual_service_network(&client).await;
    let address = network.public_ip.clone();
    let account = fixture_account(&network, "dual", "ssh", true);
    let port = account.port;
    let password = account.root_password;

    join_fixture(&client, &network).await;

    let connect =
        format!("get_shell.connect_service(\"{address}\", {port}, \"root\", \"{password}\")");
    let enabled = script_string(
        "{\"encryption_enabled\":true,\"message_encrypted_conn\":true,\"path_enc\":\"/server/conf/encode.src\",\"path_dec\":\"/server/conf/decode.bin\"}",
    );
    let disabled = script_string(
        "{\"encryption_enabled\":false,\"message_encrypted_conn\":true,\"path_enc\":\"/server/conf/encode.src\",\"path_dec\":\"/server/conf/decode.bin\"}",
    );
    let calls = [
        "get_shell.host_computer.File(\"/etc/passwd\").copy(\"/root\", \"copy\")",
        "get_shell.host_computer.File(\"/etc/passwd\").move(\"/root\", \"move\")",
        "get_shell.host_computer.File(\"/etc/passwd\").symlink(\"/root\", \"link\")",
        "get_shell.host_computer.File(\"/etc/passwd\").rename(\"other\")",
        "get_shell.host_computer.File(\"/etc/passwd\").get_content",
        "get_shell.host_computer.File(\"/etc/passwd\").set_content(\"x\")",
        "get_shell.host_computer.reboot",
        "get_shell.host_computer.show_procs",
        "get_shell.host_computer.touch(\"/root\", \"touch\")",
        "get_shell.host_computer.create_folder(\"/root\", \"folder\")",
        "mail_login(\"a@b\", \"x\")",
        "get_shell",
        "get_custom_object",
        "cd(\"/root\")",
        "user_input(\"\")",
        "poll_input",
        "clear_screen",
    ];

    for call in calls {
        let snapshot = client.network_state().await.unwrap();
        let encode = script_string(&format!(
            "Encode = function(password)\n  print {call}\n  return \"enc_\" + password\nend function"
        ));
        let script = format!(
            "remote = {connect}; pc = remote.host_computer; pc.touch(\"/server/conf\", \"encode.src\"); pc.File(\"/server/conf/encode.src\").set_content({encode}); pc.File(\"/server/conf/sshd.conf\").set_content({enabled}); print {connect}; pc.File(\"/server/conf/sshd.conf\").set_content({disabled})"
        );

        compare_in(&client, &user, &snapshot, &[script]).await;
    }

    client.shutdown();
}

/// Service installation, starting and stopping for the fixture user and for root (`ServicesIntrinsics`).
#[tokio::test]
async fn should_match_intrinsics_for_service_control() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    for name in ["libssh.so", "libhttp.so"] {
        client
            .install_library("", name, u8::try_from(Reward::Shell.code()).unwrap())
            .await
            .unwrap();
    }

    let snapshot = client.network_state().await.unwrap();
    let inner = r#"ssh = include_lib("/lib/libssh.so")
print ssh.stop_service
print ssh.start_service
print ssh.start_service
print ssh.stop_service
print ssh.install_service
print ssh.stop_service
http = include_lib("/lib/libhttp.so")
print http.install_service
print http.start_service
print http.stop_service
print http.start_service
print http.stop_service"#;
    let scripts = vec![
        r#"ssh = include_lib("/lib/libssh.so"); print ssh.install_service; print ssh.start_service; print ssh.stop_service; http = include_lib("/lib/libhttp.so"); print http.install_service; print http.start_service; print http.stop_service"#.to_owned(),
        format!(
            "root = get_shell(\"root\", \"test\"); pc = root.host_computer; pc.touch(\"/root\", \"services.src\"); pc.File(\"/root/services.src\").set_content({}); print root.build(\"/root/services.src\", \"/root\"); print root.launch(\"/root/services\")",
            script_string(inner)
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}
