//! Smart appliance fault and accident branches.

use greybel_test_support::game::Game;
use greyhack_behaviour::Reward;

use crate::support::game::{
    adminless_fridge_network, fixture_target, join_fixture, setup_player, test_client,
};
use crate::support::metaxploit::{computer_library, fixture_reply_weakness, fixture_weakness};
use crate::support::parity::compare_labeled;
use crate::support::remote::script_string;

/// A non-appliance host never produces a SmartAppliance object.
#[tokio::test]
async fn should_match_intrinsics_for_smart_appliance_wrong_type() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    client
        .install_library(
            "",
            "libsmartappliance.so",
            u8::try_from(Reward::OverrideSettings.code()).unwrap(),
        )
        .await
        .unwrap();
    let snapshot = client.network_state().await.unwrap();
    let scripts = [
        "app = include_lib(\"/lib/libsmartappliance.so\"); print app.model".to_owned(),
        "app = include_lib(\"/lib/libsmartappliance.so\"); print app.set_alarm(1)".to_owned(),
        "app = include_lib(\"/lib/libsmartappliance.so\"); print app.override_settings(1, 1)"
            .to_owned(),
    ];

    compare_labeled(
        &client,
        &user,
        &snapshot,
        "smart-appliance-wrong-type",
        &scripts,
    )
    .await;
    client.shutdown();
}

/// Broken hardware and a valid override during the five-minute accident window.
#[tokio::test]
async fn should_match_intrinsics_for_smart_appliance_faults() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let client = test_client(&game).await;
    let user = setup_player(&client).await;

    for fixture in ["broken", "timed_accident"] {
        let network = adminless_fridge_network(&client).await;
        let target = fixture_target(&network, "fridge");
        let fridge = target.local_ip.clone();
        let port = target.services[0].internal_port;
        join_fixture(&client, &network).await;
        let installed = client
            .install_fixture_library(
                &fridge,
                "libsmartappliance.so",
                vec![fixture_weakness(Reward::Shell, &[], "root", "")],
                false,
            )
            .await
            .unwrap();
        let snapshot = client.network_state().await.unwrap();
        assert!(snapshot.admin.is_none(), "smart-appliance-{fixture}");
        let library = computer_library(&snapshot, &fridge, "libsmartappliance.so");
        let (zone, weakness) = fixture_reply_weakness(&library, &installed.weaknesses[0]);
        if fixture == "timed_accident" {
            client
                .configure_appliance(&fridge, "timed_accident")
                .await
                .unwrap();
        } else {
            client.configure_appliance(&fridge, "broken").await.unwrap();
        }
        let snapshot = client.network_state().await.unwrap();
        let source = script_string(
            "app = include_lib(\"/lib/libsmartappliance.so\"); print app.model; print app.set_alarm(0); print app.override_settings(900, 200); print app.model",
        );
        let script = format!(
            "mx = include_lib(\"/lib/metaxploit.so\"); shell = mx.net_use(\"{fridge}\", {port}).dump_lib.overflow(\"{zone}\", \"{}\"); pc = shell.host_computer; pc.touch(\"/root\", \"{fixture}.src\"); pc.File(\"/root/{fixture}.src\").set_content({source}); print shell.build(\"/root/{fixture}.src\", \"/root\"); print shell.launch(\"/root/{fixture}\")",
            weakness.unsec_value
        );

        compare_labeled(&client, &user, &snapshot, fixture, &[script]).await;
    }

    client.shutdown();
}
