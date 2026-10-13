//! End-to-end tests against `greyhack-message-hook-proxy` with the test acting as the game. Verifies game-JSON
//! translation in both directions.

use std::sync::Arc;
use std::time::Duration;

use greyhack_message_hook_client::protocol::provider::CreateContextRpc;
use greyhack_message_hook_client::{BuildOutcome, BuildState};
use serde_json::Value;

use crate::support::host::{Host, game_status, is, values};
use crate::support::{
    CONTEXT_STEP_EVENTS, PID, build_replies, context_options, context_replies, prepared_build,
    received, run_line,
};

#[tokio::test]
async fn should_return_the_status_the_game_sends_on_a_health_check() {
    let (_host, client, _rx) =
        Host::connected(env!("CARGO_TARGET_TMPDIR"), Arc::new(game_status)).await;

    let status = client.health_check().await.unwrap();

    client.shutdown().await;

    assert_eq!(status.plugin_version, "1.2.3");
    assert!(status.has_player_client);
}

#[tokio::test]
async fn should_finish_a_performed_build() {
    let (_host, client, _rx) = Host::connected(
        env!("CARGO_TARGET_TMPDIR"),
        Arc::new(build_replies::<Value, Value>),
    )
    .await;
    let build = prepared_build(&client).await;

    let outcome = build
        .perform_and_wait(Some(Duration::from_secs(5)))
        .await
        .unwrap();

    client.shutdown().await;

    assert!(matches!(&outcome, BuildOutcome::Completion(_)));
    assert_eq!(outcome.state(), BuildState::Complete);
    assert_eq!(outcome.output(), "built");
}

#[tokio::test]
async fn should_dispose_a_finished_build() {
    let (_host, client, _rx) = Host::connected(
        env!("CARGO_TARGET_TMPDIR"),
        Arc::new(build_replies::<Value, Value>),
    )
    .await;
    let build = prepared_build(&client).await;

    build
        .perform_and_wait(Some(Duration::from_secs(5)))
        .await
        .unwrap();

    let disposed = build.dispose().await.unwrap();

    client.shutdown().await;

    assert!(disposed.dispose_success);
}

#[tokio::test]
async fn should_create_a_context_on_the_games_terminal() {
    let (_host, client, _rx) = Host::connected(
        env!("CARGO_TARGET_TMPDIR"),
        Arc::new(context_replies::<Value, Value>),
    )
    .await;

    let context = client.create_context(context_options()).await.unwrap();

    client.shutdown().await;

    assert_eq!(context.terminal_pid(), PID);
}

#[tokio::test]
async fn should_deliver_the_events_of_a_context() {
    let (_host, client, mut rx) = Host::connected(
        env!("CARGO_TARGET_TMPDIR"),
        Arc::new(context_replies::<Value, Value>),
    )
    .await;
    let context = client.create_context(context_options()).await.unwrap();

    let names = run_line(&context).await;
    let game = received(&mut rx, 2).await;

    client.shutdown().await;

    assert_eq!(names, CONTEXT_STEP_EVENTS);
    assert!(is::<CreateContextRpc>(&game[0]));
    assert_eq!(
        values::<CreateContextRpc>(&game[0])["program_name"],
        "myprogram"
    );
}
