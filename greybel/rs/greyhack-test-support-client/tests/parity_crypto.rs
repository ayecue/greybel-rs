//! crypto: Wi-Fi captures and cracking, and SMTP user lists.

use greybel_test_support::game::Game;
use greyhack_test_support_client::protocol::test_support::LanNodeKind;

use crate::support::game::{join_fixture, public_smtp_network, setup_player, test_client};
use crate::support::metaxploit::player;
use crate::support::parity::compare_in;
use crate::support::remote::fixture_account;

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
