use greybel_test_support::game::Game;
use greyhack_behaviour::Requirement;

use crate::support::game::{kernel_content, setup_player, test_client};

#[tokio::test]
async fn should_write_query_execute_and_create_a_network_through_the_bridge() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let test = test_client(&game).await;
    let user = setup_player(&test).await;
    let home = format!("/home/{user}");
    let path = format!("{home}/bridge-{}.txt", std::process::id());

    let written = test.write_file(&path, "bridge-state").await.unwrap();
    let query = test.file_state(&path).await.unwrap();
    let execution = test
        .execute(
            "print(\"bridge-execution\")",
            "/project/test.src",
            "/project",
            "test",
        )
        .await
        .unwrap();
    let network = test.create_network("45.45.45.45").await.unwrap();

    assert_eq!(written.path, path);
    assert!(query.exists);
    assert_eq!(query.content, "bridge-state");
    assert!(!execution.failed);
    assert!(execution.output.contains("bridge-execution"));
    assert_eq!(network.public_ip, "45.45.45.45");
    assert!(!network.local_ip.is_empty());

    test.shutdown();
}

/// A router kernel a test replaced and didn't put back is restored by the next player setup.
#[tokio::test]
async fn should_restore_a_replaced_router_kernel_at_the_next_setup() {
    let game = Game::start(env!("CARGO_TARGET_TMPDIR"));
    let test = test_client(&game).await;

    setup_player(&test).await;

    let requirement = u8::try_from(Requirement::ConnGateway.code()).unwrap();

    let main_router = test.network_state().await.unwrap().nodes[0]
        .local_ip
        .to_string();
    let original = test
        .set_router_kernel_requirement(&main_router, requirement)
        .await
        .unwrap();
    let replaced = kernel_content(&test).await;

    setup_player(&test).await;

    let restored = kernel_content(&test).await;

    assert_ne!(replaced, original.original);
    assert_eq!(restored, original.original);
    test.shutdown();
}
