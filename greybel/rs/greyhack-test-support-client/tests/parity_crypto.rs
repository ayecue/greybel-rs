//! crypto: Wi-Fi captures and cracking, and SMTP user lists.

use greybel_test_support::game::Game;
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{
    dual_service_network, join_fixture, join_network, public_smtp_network, setup_player,
    ssh_network_of_kind, test_client,
};
use crate::support::metaxploit::player;
use crate::support::parity::{compare_in, compare_labeled, compare_labeled_with_networks};
use crate::support::remote::{fixture_account, script_string};

/// Crypto file operations, password deciphering, Wi-Fi capture/cracking and SMTP enumeration on mirrored networks.
#[tokio::test]
async fn should_match_intrinsics_for_crypto_wifi_and_smtp() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let snapshot = client.network_state().await.unwrap();
    let (bssid, essid, wifi_password) = snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Router(router) if node.parent.is_none() => Some((
                router.bssid.as_ref()?.clone(),
                router.essid.as_ref()?.clone(),
                router.wifi_password.clone(),
            )),
            _ => None,
        })
        .unwrap();
    let password = player(&snapshot)
        .users
        .iter()
        .find(|user| user.name == "root")
        .unwrap()
        .password
        .clone();
    let scripts = vec![
        "crypto = include_lib(\"/lib/crypto.so\"); home = get_shell.host_computer; home.touch(home_dir, \"secret.txt\"); file = home_dir + \"/secret.txt\"; home.File(file).set_content(\"classified\"); print crypto.is_encrypted(file); print crypto.encrypt(file, \"hunter2\"); print crypto.is_encrypted(file); print crypto.encrypt(file, \"hunter2\"); print crypto.decrypt(file, \"wrong1\"); print crypto.decrypt(file, \"hunter2\"); print crypto.is_encrypted(file); print crypto.encrypt(file, \"bad pass\"); print crypto.encrypt(file, \"0123456789abcdef1\"); print crypto.encrypt(\"\", \"abc\"); print crypto.encrypt(home_dir + \"/missing\", \"abc\"); print crypto.is_encrypted(home_dir + \"/missing\")"
            .to_owned(),
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.airmon(\"start\", \"wlan0\"); print crypto.airmon(\"start\", \"eth9\"); print crypto.aireplay(\"{bssid}\", \"{essid}\", 1); print crypto.aircrack(home_dir + \"/file.cap\"); print crypto.aireplay(\"00:00:00:00:00:00\", \"nope\")"
        ),
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.airmon(\"start\", \"wlan0\"); print crypto.aireplay(\"{bssid}\", \"{essid}\", 3000); print crypto.aircrack(home_dir + \"/file.cap\"); print crypto.aircrack(home_dir + \"/file.cap\") == \"{wifi_password}\""
        ),
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.decipher(md5(\"{password}\"))"
        ),
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.smtp_user_list(\"bad\", 25); print crypto.smtp_user_list(\"{address}\", 25)",
            address = snapshot.public_ip
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;

    let network = public_smtp_network(&client).await;
    let local = fixture_account(&network, "smtp", "smtp", false);
    let public = fixture_account(&network, "smtp", "smtp", true);
    let local_ip = local.address;
    let port = local.port;

    let snapshot = join_fixture(&client, &network).await;
    let public_port = public.port;
    let public_ip = &snapshot.public_ip;
    let scripts = vec![
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.smtp_user_list(\"{local_ip}\", {port})"
        ),
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.smtp_user_list(\"{local_ip}\", 22)"
        ),
        format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.smtp_user_list(\"{public_ip}\", {public_port})"
        ),
    ];

    compare_in(&client, &user, &snapshot, &scripts).await;
    client.shutdown();
}

/// Encryption validation, permissions, malformed captures, missing routers, and null helper cases.
#[tokio::test]
async fn should_match_intrinsics_for_crypto_file_validation() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client.configure_computer("", "crypto_files").await.unwrap();

    let snapshot = client.network_state().await.unwrap();
    let scripts = vec![
        "crypto = include_lib(\"/lib/crypto.so\"); print [encrypt({}, \"x\", \"abc\"), decrypt({}, \"x\", \"abc\"), is_encrypted({}, \"x\"), aircrack({}, \"x\"), smtp_user_list({}, \"1.2.3.4\", 25), airmon({}, \"start\", \"wlan0\"), aireplay({}, \"x\", \"y\")]; print [crypto.encrypt(1, \"abc\"), crypto.decrypt(1, \"abc\"), crypto.is_encrypted(1), crypto.aircrack(1), crypto.smtp_user_list(1, 25)]; print crypto.decipher(\"bad\")".to_owned(),
        "crypto = include_lib(\"/lib/crypto.so\"); f = home_dir + \"/plain.txt\"; print crypto.encrypt(f, \"abc\"); print crypto.encrypt(f, \"abc\"); print crypto.decrypt(f, \"bad\"); print crypto.decrypt(f, \"abc\"); print crypto.decrypt(f, \"abc\"); print crypto.encrypt(home_dir + \"/missing\", \"abc\"); print crypto.decrypt(home_dir + \"/missing\", \"abc\"); print crypto.is_encrypted(home_dir + \"/missing\"); print crypto.decrypt(\"\", \"abc\"); print crypto.is_encrypted(\"\")".to_owned(),
        "crypto = include_lib(\"/lib/crypto.so\"); f = home_dir + \"/noaccess/locked.cap\"; print crypto.encrypt(f, \"abc\"); print crypto.decrypt(f, \"abc\"); print crypto.is_encrypted(f); print crypto.aircrack(f)".to_owned(),
        "crypto = include_lib(\"/lib/crypto.so\"); print crypto.aircrack(home_dir + \"/plain.txt\"); print crypto.aircrack(home_dir + \"/malformed.cap\"); print crypto.aircrack(home_dir + \"/encrypted.cap\"); print crypto.aircrack(home_dir + \"/missing_router.cap\")".to_owned(),
        "get_shell.host_computer.File(\"/lib/init.so\").delete; crypto = include_lib(\"/lib/crypto.so\"); print crypto.encrypt(home_dir + \"/plain.txt\", \"abc\"); print crypto.decrypt(home_dir + \"/plain.txt\", \"abc\"); print crypto.is_encrypted(home_dir + \"/plain.txt\")".to_owned(),
    ];

    compare_labeled(&client, &user, &snapshot, "crypto-files", &scripts).await;
    client.shutdown();
}

/// Monitor-mode support, disabled cards, wrong devices, and the aireplay ordering before monitor mode.
#[tokio::test]
async fn should_match_intrinsics_for_crypto_hardware_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for fixture in ["no_monitor", "stop", "ethernet", "unsupported", "disabled"] {
        let user = setup_player(&client).await;

        if fixture == "ethernet" {
            client
                .configure_computer("", "ethernet_home")
                .await
                .unwrap();
        } else if fixture == "unsupported" {
            client
                .configure_computer("", "unsupported_monitor")
                .await
                .unwrap();
        } else if fixture == "disabled" {
            client
                .configure_computer("", "disabled_wifi")
                .await
                .unwrap();
        }

        let snapshot = client.network_state().await.unwrap();
        let (bssid, essid) = snapshot
            .nodes
            .iter()
            .find_map(|node| match &node.node {
                LanNodeKind::Router(router) if node.parent.is_none() => {
                    Some((router.bssid.clone()?, router.essid.clone()?))
                }
                _ => None,
            })
            .unwrap();
        let script = match fixture {
            "no_monitor" => format!(
                "crypto = include_lib(\"/lib/crypto.so\"); print crypto.aireplay(\"{bssid}\", \"{essid}\", 1)"
            ),
            "stop" => format!(
                "crypto = include_lib(\"/lib/crypto.so\"); print crypto.airmon(\"start\", \"wlan0\"); print crypto.airmon(\"stop\", \"wlan0\"); print crypto.aireplay(\"{bssid}\", \"{essid}\", 1)"
            ),
            "ethernet" => "crypto = include_lib(\"/lib/crypto.so\"); print crypto.airmon(\"start\", \"eth0\")".to_owned(),
            "unsupported" => "crypto = include_lib(\"/lib/crypto.so\"); print crypto.airmon(\"start\", \"wlan0\")".to_owned(),
            "disabled" => format!(
                "crypto = include_lib(\"/lib/crypto.so\"); print crypto.airmon(\"start\", \"wlan0\"); print crypto.aireplay(\"{bssid}\", \"{essid}\", 1)"
            ),
            _ => unreachable!(),
        };

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("crypto-hardware:{fixture}"),
            &[script],
        )
        .await;
    }

    client.shutdown();
}

/// Pending SSH encryption rejects decipher before it reads the hash.
#[tokio::test]
async fn should_match_intrinsics_for_decipher_during_ssh_encryption() {
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
        "Encode = function(password)\n  print include_lib(\"/lib/crypto.so\").decipher(md5(\"x\"))\n  return \"enc_\" + password\nend function",
    );
    let config = script_string(
        "{\"encryption_enabled\":true,\"path_enc\":\"/server/conf/encode.src\",\"path_dec\":\"/server/conf/decode.bin\"}",
    );
    let script = format!(
        "remote = {connect}; pc = remote.host_computer; pc.touch(\"/server/conf\", \"encode.src\"); pc.File(\"/server/conf/encode.src\").set_content({encode}); pc.File(\"/server/conf/sshd.conf\").set_content({config}); print {connect}"
    );

    compare_labeled(&client, &user, &snapshot, "decipher-ssh", &[script]).await;

    let user = setup_player(&client).await;
    let home = client.network_state().await.unwrap();
    let target = ssh_network_of_kind(&client, "RentServer").await;
    let target_snapshot = join_fixture(&client, &target).await;
    let (bssid, essid) = target_snapshot
        .nodes
        .iter()
        .find_map(|node| match &node.node {
            LanNodeKind::Router(router) if node.parent.is_none() => {
                Some((router.bssid.clone()?, router.essid.clone()?))
            }
            _ => None,
        })
        .unwrap();

    client
        .configure_computer(&target.main_local_ip, "distant_router")
        .await
        .unwrap();

    let target_snapshot = client.network_state().await.unwrap();

    join_network(&client, &home.public_ip).await;

    let base = client.network_state().await.unwrap();
    let script = format!(
        "crypto = include_lib(\"/lib/crypto.so\"); print crypto.aireplay(\"{bssid}\", \"{essid}\", 1)"
    );

    compare_labeled_with_networks(
        &client,
        &user,
        &base,
        &[target_snapshot],
        "crypto-hardware:distant",
        &script,
    )
    .await;
    client.shutdown();
}

/// Every `PlayerUtils.GetSmtpServer` error, using public or LAN addresses as its validation order requires.
#[tokio::test]
async fn should_match_intrinsics_for_smtp_helper_errors() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;

    for fixture in [
        "unknown_public",
        "missing_public_port",
        "missing_lan_host",
        "host_down",
        "invalid_target",
        "service_not_found",
    ] {
        let user = setup_player(&client).await;
        let network = public_smtp_network(&client).await;
        let public = fixture_account(&network, "smtp", "smtp", true);
        let local = fixture_account(&network, "smtp", "smtp", false);

        join_fixture(&client, &network).await;

        let (address, port) = match fixture {
            "unknown_public" => ("240.0.0.1".to_owned(), public.port),
            "missing_public_port" => (public.address.clone(), 1),
            "missing_lan_host" => ("192.168.0.99".to_owned(), local.port),
            "host_down" => {
                client.configure_computer("", "rebooting").await.unwrap();

                (local.address.clone(), local.port)
            }
            "invalid_target" => {
                client
                    .configure_computer(&local.address, "invalid_smtp_target")
                    .await
                    .unwrap();

                (public.address.clone(), public.port)
            }
            "service_not_found" => {
                client
                    .configure_computer(&local.address, "missing_smtp_service")
                    .await
                    .unwrap();

                (local.address.clone(), local.port)
            }
            _ => unreachable!(),
        };
        let snapshot = client.network_state().await.unwrap();
        let script = format!(
            "crypto = include_lib(\"/lib/crypto.so\"); print crypto.smtp_user_list(\"{address}\", {port})"
        );

        compare_labeled(
            &client,
            &user,
            &snapshot,
            &format!("smtp-helper:{fixture}"),
            &[script],
        )
        .await;
    }

    client.shutdown();
}
