//! Service installation and configuration failures.

use greybel_test_support::game::Game;
use greyhack_behaviour::{LibraryKind, Reward};

use crate::support::game::{
    fixture_target, join_fixture, router_ssh_network, setup_player, test_client,
};
use crate::support::parity::compare_labeled;
use crate::support::remote::{fixture_account, script_string};

/// Repository configuration is valid on a computer; detached and malformed service receivers are not.
#[tokio::test]
async fn should_match_intrinsics_for_repository_and_wrong_self_services() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            &LibraryKind::Repository.file_name(),
            u8::try_from(Reward::Shell.code()).unwrap(),
        )
        .await
        .unwrap();

    let snapshot = client.network_state().await.unwrap();
    let scripts = [
        "repo = include_lib(\"/lib/librepository.so\"); print repo.install_service; print repo.start_service; print repo.stop_service".to_owned(),
        "print install_service(1); print start_service(1); print stop_service(1)".to_owned(),
        "print install_service({\"classID\":\"Service\"})".to_owned(),
        "print start_service({\"classID\":\"Service\"})".to_owned(),
        "print stop_service({\"classID\":\"Service\"})".to_owned(),
    ];

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "services-repository-and-wrong-self",
        &scripts,
    )
    .await;
    client.shutdown();
}

/// A program hosted on the main router gets the HTTP router-specific service errors.
#[tokio::test]
async fn should_match_intrinsics_for_router_http_service_control() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = router_ssh_network(&client, false).await;

    join_fixture(&client, &network).await;
    client.configure_computer("", "router_ssh").await.unwrap();
    let target = fixture_target(&network, "ssh");
    let local_ip = target.local_ip.clone();
    let account = fixture_account(&network, "ssh", "ssh", true);
    let public_ip = account.address.clone();
    let password = account.root_password.clone();
    client
        .install_fixture_library(
            &local_ip,
            "libhttp.so",
            vec![crate::support::metaxploit::fixture_weakness(
                Reward::Shell,
                &[],
                "root",
                "",
            )],
            false,
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let source = script_string(
        "http = include_lib(\"/lib/libhttp.so\"); print typeof(http); print http.install_service; print http.start_service; print http.stop_service",
    );
    let script = format!(
        "remote = get_shell.connect_service(\"{}\", {}, \"root\", \"{}\"); print typeof(remote); pc = remote.host_computer; print pc.local_ip; print pc.File(\"/lib/libhttp.so\") != null; pc.touch(\"/root\", \"http.src\"); pc.File(\"/root/http.src\").set_content({}); print remote.build(\"/root/http.src\", \"/root\"); print remote.launch(\"/root/http\")",
        public_ip, account.port, password, source
    );

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "router-http-service-control",
        &[script],
    )
    .await;
    client.shutdown();
}

/// A CTF object created before its event is deleted returns null from every read.
#[tokio::test]
async fn should_match_intrinsics_for_a_deleted_ctf_event() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;
    let network = crate::support::game::smtp_network(&client).await;

    join_fixture(&client, &network).await;
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("DeletedCtf{suffix}");
    client
        .setup_blockchain("Gb8134", &format!("ctfwallet{suffix}"))
        .await
        .unwrap();
    client.setup_mail("ctfplayer", "mailpass").await.unwrap();
    client.setup_ctf(&name).await.unwrap();
    let snapshot = client.network_state().await.unwrap();
    let ctf = snapshot.ctf.as_ref().unwrap();
    let creator = script_string(&ctf.user);
    let event = script_string(&ctf.name);
    let script = format!(
        "ctf = get_ctf({creator}, \"oldpass\", {event}); print typeof(ctf); user_input(\"delete\"); print ctf.get_description; print ctf.get_template; print ctf.get_creator_name; print ctf.get_mail_content"
    );
    let event_name = ctf.name.clone();
    let mirror_event = event_name.clone();
    let mirror_action = std::sync::Arc::new(move |world: &greyhack_world::World| {
        greybel_test_support::future::block_on(world.database().conn.execute(
            "DELETE FROM ctf_events WHERE name = ?1",
            [mirror_event.as_str()],
        ))
        .unwrap();
    });
    let game_event = event_name.clone();
    let game_client = client.clone();
    let inputs = ["delete".to_owned()];

    crate::support::parity::compare_interactive_action(
        &client,
        &user,
        &snapshot,
        &script,
        &inputs,
        mirror_action,
        move || {
            let client = game_client.clone();
            let event_name = game_event.clone();
            async move {
                client.delete_ctf(&event_name).await.unwrap();
            }
        },
    )
    .await;
    client.shutdown();
}
