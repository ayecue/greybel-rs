//! The player's computer: files, users and groups, addresses, reboots, startup files and services.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;

use crate::support::game::{join_network, setup_player, test_client};
use crate::support::parity::{compare_in, game_outcomes, intrinsics_outcomes};
use crate::support::remote::{forwarded_root, script_string, service_account};

/// The `File` class on files the script makes itself. A copy or move isn't read back at once: the game updates what the
/// script sees of it some time after the call returns.
#[tokio::test]
async fn should_match_intrinsics_for_file_operations() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    // Each script makes its own folder: the game may still show the previous script's folder, deleted at its end, for
    // a moment after that script finished, so a shared name would let one script see another's leftovers.
    let prelude = |index: usize| {
        format!(
            "pc = get_shell.host_computer; pc.create_folder(home_dir, \"probe{index}\"); d = pc.File(home_dir + \"/probe{index}\"); pc.touch(d.path, \"a.txt\"); f = pc.File(d.path + \"/a.txt\")"
        )
    };
    let scripts = [
        "print [d.name, d.path, d.is_folder, d.is_binary, d.is_symlink, d.size, d.permissions, d.owner, d.group, d.parent.path, d.has_permission(\"r\"), d.has_permission(\"x\"), d.has_permission(\"q\")]; print [f.name, f.size, f.permissions, f.owner, f.group, f.get_content, f.is_folder, f.allow_import, f.get_files, f.get_folders]; print f.set_content(\"hello\"); print [f.size, f.get_content]; print d.get_files.len; print d.get_folders; print typeof(d.get_files[0])",
        "print f.chmod(\"u-w\"); print f.permissions; print f.chmod(\"o+rwx\"); print f.permissions; print f.chmod(\"a-x\"); print f.permissions; print f.chmod(\"bad\"); print f.chmod(\"u+w\", 1); print f.chmod(\"g=r\"); print f.chmod(\"777\"); print f.permissions",
        "print f.copy(d.path, \"b.txt\"); f = pc.File(f.path); print f.copy(\"/nope\", \"c.txt\"); print f.copy(d.path, \"\"); print f.rename(\"\"); print f.rename(\"a/b\")",
        "print f.move(d.path, \"m.txt\")",
        "print f.rename(\"r.txt\"); print f.path",
        "print f.set_owner(\"root\"); print f.owner; print f.set_owner(\"nobody\"); print f.set_group(\"root\"); print f.group; print f.set_group(\"nogroup\"); print d.set_owner(\"root\", 1); print d.set_group(\"guest\", 1); print f.delete; print f.delete",
        "print pc.touch(d.path, \"a.txt\"); print pc.touch(\"/nope\", \"x\"); print pc.touch(d.path, \"\"); print pc.touch(d.path, \"bad name!\"); print pc.create_folder(d.path, \"sub\"); print pc.create_folder(d.path, \"sub\"); print pc.create_folder(\"/nope\", \"x\"); print pc.create_folder(d.path, \"\"); print pc.File(\"/nope\"); print pc.File(\"relative\"); print pc.File(\"\")",
        "print pc.File(\"/bin/ls\").get_content; print pc.File(\"/bin/ls\").set_content(\"x\"); print pc.File(\"/bin/ls\").allow_import; print pc.File(\"/etc/passwd\").permissions; print pc.File(\"/\").name; print pc.File(\"/\").parent; print pc.File(\"/\").permissions; print pc.File(\"/root\").permissions; print pc.File(home_dir).permissions; print pc.File(home_dir + \"/Config\").permissions; print pc.File(\"/home/guest/Config\").permissions",
    ]
    .iter()
    .enumerate()
    .map(|(index, script)| format!("{}\n{script}\nd.delete", prelude(index)))
    .collect::<Vec<_>>();
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
        "print command_info(\"LS_USAGE\"); print command_info(\"nope\"); print command_info(\"\")",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let snapshot = client.network_state().await.unwrap();

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
    let network = client
        .find_new_network("Supermercados", "ssh")
        .await
        .unwrap();

    join_network(&client, &network.public_ip).await;

    let snapshot = client.network_state().await.unwrap();
    let account = service_account(&snapshot, "ssh");
    let address = network.public_ip.to_string();
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
