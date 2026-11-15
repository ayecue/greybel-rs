//! Game objects: methods called detached, shared method names, and object identity.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;

use crate::support::game::{ftp_network, join_fixture, setup_player, smtp_network, test_client};
use crate::support::parity::{
    assert_succeeded, compare_in, compare_pair_in, game_outcomes, snapshot_outcomes,
};
use crate::support::remote::{fixture_account, rshell_server_network, script_string};

/// Game methods are also globals taking the object first. Their `self` checks accept any game map, or only an object
/// whose type has the method key where the game requests it; a missing helper then returns null or text, or crashes, in
/// the game's check order.
#[tokio::test]
async fn should_match_intrinsics_for_detached_methods() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = vec![
        "pc = host_computer(get_shell); print [typeof(pc), pc == get_shell.host_computer, @host_computer == @get_shell.host_computer, lan_ip(pc) == pc.local_ip, public_ip_pc(pc) == pc.public_ip, get_name(pc) == pc.get_name, @lan_ip == @pc.local_ip, @public_ip_pc == @pc.public_ip]".to_owned(),
        "pc = host_computer(get_shell); root = File(pc, \"/\"); print [path(root), name(File(pc, \"/bin\")), parent(File(pc, \"/bin\")) == root, is_folder(root), permissions(root), owner(root), get_files(File(pc, \"/etc\")).len == File(pc, \"/etc\").get_files.len, @File == @pc.File, @get_files == @root.get_files]".to_owned(),
        "r = get_router; print [typeof(r), local_ip(r) == r.local_ip, public_ip(r) == r.public_ip, used_ports(r).len == r.used_ports.len, devices_lan_ip(r) == r.devices_lan_ip, @used_ports == @r.used_ports]".to_owned(),
        "meta = include_lib(\"/lib/metaxploit.so\"); lib = load(meta, \"/lib/init.so\"); print [typeof(lib), lib_name(lib) == lib.lib_name, version(lib) == lib.version, scan(meta, lib).len, @scan == @meta.scan, @overflow == @lib.overflow]".to_owned(),
        "pc = host_computer(get_shell); print [host_computer(\"text\"), host_computer(null), host_computer(1), host_computer({}), host_computer(pc)]".to_owned(),
        "pc = host_computer(get_shell); print [lan_ip(get_shell), File(get_shell, \"/\"), name(pc), path(get_router), local_ip(pc), public_ip(pc)]".to_owned(),
        "copy = new get_shell; print [host_computer(copy), copy.host_computer]; print host_computer".to_owned(),
        "print [typeof(@flood_connection), typeof(@get_balance_subwallet), typeof(@reset_password_coin), typeof(@scan_debuglib), typeof(@delete_mail)]".to_owned(),
        "print delete(host_computer(get_shell))".to_owned(),
        "print @local_ip == @get_router.local_ip".to_owned(),
        "print [public_ip(get_shell), public_ip(get_shell.__isa), public_ip({}), public_ip(1), port_number({}), port_number(1)]".to_owned(),
        "pc = get_shell.host_computer; r = get_router; print [lan_ip(r), local_ip(pc), public_ip_pc(r), public_ip(pc)]".to_owned(),
        "print [load(get_shell, \"/lib/init.so\"), decipher(get_shell, \"hash\"), overflow(get_shell, \"0x0\", \"buffer\"), load(get_shell, \"\"), overflow({}, \"0x0\", \"buffer\")]".to_owned(),
        "meta = include_lib(\"/lib/metaxploit.so\"); print [scan_debuglib(get_shell), scan_debuglib(get_shell.__isa), scan_debuglib(meta), scan_debuglib({}), scan_debuglib(1)]".to_owned(),
        "print [launch(get_shell.host_computer, \"/bin/ls\"), launch({}, \"/bin/ls\")]".to_owned(),
        "print [aircrack(get_shell, \"\"), airmon(get_shell, \"\", \"eth0\"), aireplay(get_shell, \"\", \"\", 1), smtp_user_list(get_shell, \"bad\", 25), encrypt(get_shell, \"/x\", \"bad!\"), rshell_client(get_shell, \"bad\", 1222, \"x\")]".to_owned(),
        "print [port_info(get_shell, get_shell), scan(get_shell, get_shell)]; print net_use(get_shell, \"bad\", 22)".to_owned(),
    ];

    assert_succeeded(&compare_in(&client, &user, &snapshot, &scripts).await);
    client.shutdown();
}

/// Every shared type-map key accepts the other class's object before the called intrinsic resolves its own helper.
#[tokio::test]
async fn should_match_intrinsics_for_detached_shared_keys() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let mail_network = smtp_network(&client).await;

    join_fixture(&client, &mail_network).await;
    client.setup_mail("parity", "mailpass1").await.unwrap();
    client
        .install_library(
            "",
            "blockchain.so",
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let suffix = std::process::id() % 10000;
    let coin = format!("Gb{suffix}");
    let wallet = format!("n{suffix}");
    let ftp_network = ftp_network(&client).await;
    let ftp_account = fixture_account(&ftp_network, "ftp", "ftp", true);

    join_fixture(&client, &ftp_network).await;
    client.setup_blockchain(&coin, &wallet).await.unwrap();
    client.freeze_clock().await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let mail = snapshot.player_mail.as_ref().unwrap().address.clone();
    let public_ip = snapshot.public_ip.clone();
    let ftp_port = ftp_account.port;
    let ftp_password = ftp_account.root_password;
    let mail_login = format!("mail_login(\"{mail}\", \"mailpass1\")");
    let scripts = vec![
        format!(
            "mail = {mail_login}; f = get_shell.host_computer.File(\"/\"); print [delete(mail), delete_subwallet(mail), delete_subwallet(f)]"
        ),
        format!(
            "chain = include_lib(\"/lib/blockchain.so\"); w = create_wallet(chain, \"{wallet}\", \"matrix1\"); c = get_coin(chain, \"{coin}\", \"{user}\", \"matrix1\"); print create_subwallet(c, \"{wallet}\", get_pin(w), \"alpha\", \"pass1\"); a = get_subwallet(c, \"alpha\"); print [typeof(w), typeof(c), typeof(a)]; print [reset_password_coin(w, \"newpass\"), reset_password(c, \"newpass\"), get_balance(a, \"{coin}\"), get_balance_subwallet(w), delete(a), delete(w), buy_coin(get_shell, \"{coin}\", 1, 1, \"alpha\"), sell_coin(get_shell, \"{coin}\", 1, 1, \"alpha\")]"
        ),
        format!("mail = {mail_login}; fetch(mail); print fetch(get_shell); print fetch({{}})"),
        format!(
            "ftp = connect_service(get_shell, \"{public_ip}\", {ftp_port}, \"root\", \"{ftp_password}\", \"ftp\"); pc = host_computer(ftp); f = File(pc, \"/\"); normal = get_shell.host_computer.File(\"/\"); print [typeof(ftp), typeof(pc), typeof(f)]; print [@File == @pc.File, @create_folder == @pc.create_folder, @get_name == @pc.get_name, @scp == @ftp.scp, @host_computer == @ftp.host_computer]; print [@normal.copy == @f.copy, @normal.move == @f.move, @normal.rename == @f.rename, @normal.path == @f.path, @normal.parent == @f.parent, @normal.name == @f.name, @normal.is_folder == @f.is_folder, @normal.is_binary == @f.is_binary, @normal.is_symlink == @f.is_symlink, @normal.has_permission == @f.has_permission, @normal.delete == @f.delete, @normal.get_folders == @f.get_folders, @normal.get_files == @f.get_files, @normal.permissions == @f.permissions, @normal.owner == @f.owner, @normal.group == @f.group, @normal.size == @f.size, @normal.date == @f.date]"
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// A wrong game map crashes only when the intrinsic dereferences its missing helper. Each crash is a separate script;
/// an unknown scan address returns before that dereference.
#[tokio::test]
async fn should_match_intrinsics_for_detached_method_crashes() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let mail_network = smtp_network(&client).await;

    join_fixture(&client, &mail_network).await;
    client.setup_mail("parity", "mailpass1").await.unwrap();
    client.setup_debug().await.unwrap();
    client.freeze_clock().await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let mail = snapshot.player_mail.as_ref().unwrap().address.clone();
    let debug = snapshot.debug.as_ref().unwrap();
    let debug_user = script_string(&debug.user);
    let debug_password = script_string(&debug.password);
    let mail_login = format!("mail_login(\"{mail}\", \"mailpass1\")");
    let valid_library =
        "meta = include_lib(\"/lib/metaxploit.so\"); lib = load(meta, \"/lib/init.so\")";
    let scripts = vec![
        "port_number(get_router)".to_owned(),
        "port_number(get_router.__isa)".to_owned(),
        "port_info(get_shell, get_router.used_ports[0])".to_owned(),
        "aircrack(get_shell, \"/capture.cap\")".to_owned(),
        "rshell_remove(get_shell)".to_owned(),
        "rshell_server(get_shell)".to_owned(),
        format!("{valid_library}; scan(get_shell, lib)"),
        format!("{valid_library}; scan_address(get_shell, lib, \"0x0\")"),
        format!(
            "{valid_library}; debug = lib.debug_tools({debug_user}, {debug_password}); scan(debug, lib)"
        ),
        format!("{mail_login}; wait(2.1); fetch(get_shell)"),
        format!(
            "{mail_login}; f = get_shell.host_computer.File(\"/\"); wait(2.1); delete_mail(f, [])"
        ),
        format!("{mail_login}; wait(2.1); delete_all(get_shell)"),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;

    client
        .disable_network(&snapshot.public_ip, true)
        .await
        .unwrap();

    let offline = client.network_state().await.unwrap();
    let scripts = vec![
        "mx = include_lib(\"/lib/metaxploit.so\"); print mx.rshell_remove(1)".to_owned(),
        "net_use(get_shell, \"8.8.8.8\", 22)".to_owned(),
        "net_use(get_shell, \"bad\", 22)".to_owned(),
    ];

    compare_in(&client, &user, &offline, &scripts).await;
    client.shutdown();
}

/// Reverse shells of one computer are separate objects, one per client process, and the same ones each time the server
/// lists them; their `host_computer` is one object (`HelperScript.AddShell`, `HelperScript.AddComputer`).
#[tokio::test]
async fn should_match_intrinsics_for_reverse_shell_identities() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = rshell_server_network(&client, "RentServer").await;
    let server = r#"mx = include_lib("/lib/metaxploit.so")
wait(3)
a = mx.rshell_server
b = mx.rshell_server
print a.len
print a[0] == b[0]
print a[0] == a[1]
print a[0].host_computer == a[1].host_computer"#;
    let connecting = format!(
        r#"wait(0.5)
mx = include_lib("/lib/metaxploit.so")
print mx.rshell_client("{0}", 1222, "client")
print mx.rshell_client("{0}", 1222, "other")
wait(5)"#,
        snapshot.public_ip
    );

    compare_pair_in(&client, &user, &snapshot, [server, &connecting]).await;
    client.shutdown();
}

/// Objects the game gives again for the same thing are one object: ports by their values (`Puerto.Equals`), routers,
/// shells, computers and files by what they reach (`HelperScript`'s `Add*` methods).
#[tokio::test]
async fn should_match_intrinsics_for_object_identities() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = [
        "r = get_router; a = r.used_ports; b = r.used_ports; print a.len > 1; print a[0] == b[0]; print a[0] == a[1]",
        "a = get_router.used_ports; b = get_router.used_ports; print a[0] == b[0]; print get_router == get_router",
        "r = get_router; a = r.used_ports; print r.ping_port(a[0].port_number) == a[0]",
        "r = get_router; a = r.used_ports; m = {}; m[a[0]] = 1; m[r.used_ports[0]] = 2; print m.len",
        "print get_shell == get_shell; print get_shell.host_computer == get_shell.host_computer; print get_shell.host_computer.File(\"/bin\") == get_shell.host_computer.File(\"/bin\")",
    ];
    let expected = snapshot_outcomes(&scripts, &user, &snapshot);

    let actual = game_outcomes(&client, &scripts).await;

    assert_eq!(actual, expected);
    client.shutdown();
}

/// `include_lib` takes a library's kind from its content, not its file name, and gives services only for the SSH,
/// FTP, HTTP, rshell and repository libraries (`PlayerIntrinsics.include_lib`).
#[tokio::test]
async fn should_match_intrinsics_for_including_a_library_by_its_content() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let scripts = vec![
        "root = get_shell(\"root\", \"test\"); print root.host_computer.File(\"/lib/metaxploit.so\").rename(\"tools.so\"); print [typeof(include_lib(\"/lib/tools.so\")), typeof(include_lib(\"/lib/metaxploit.so\")), typeof(include_lib(\"/lib/init.so\")), typeof(include_lib(\"/lib/kernel_module.so\")), typeof(include_lib(\"/bin/ls\"))]".to_owned(),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}
