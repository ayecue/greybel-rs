//! End-to-end tests against an in-process mock plugin server.

use std::sync::Arc;
use std::time::Duration;

use greyhack_message_hook_client::protocol::provider::MessageHookNotification;
use greyhack_message_hook_client::{
    BuildOutcome, BuildState, Client, ClientError, ClientOptions, GreybelNotification, Payload,
};
use tokio::net::TcpListener;

use crate::support::{
    CONTEXT_STEP_EVENTS, PID, build_handler, connected, connected_context, context_handler,
    context_options, create_build, eager_context_handler, options, prepared_build, received,
    run_line, status,
};

#[tokio::test]
async fn should_return_the_status_on_a_health_check() {
    let (client, _rx) = connected(Arc::new(|_| vec![status()]), None).await;
    let status = client.health_check().await.unwrap();

    client.shutdown().await;
    assert_eq!(status.plugin_version, "1.2.3");
    assert!(status.has_player_client);
}

#[tokio::test]
async fn should_fail_to_connect_after_retries() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    drop(listener);

    let options = ClientOptions {
        connect_attempts: 2,
        ..options(port)
    };
    let error = Client::connect(options).await.unwrap_err();

    assert!(
        matches!(error, ClientError::Connect { attempts: 2, .. }),
        "{error}"
    );
}

#[tokio::test]
async fn should_create_a_build_with_a_generated_id() {
    let (client, _rx) = connected(Arc::new(build_handler), None).await;
    let build = create_build(&client).await;

    client.shutdown().await;
    assert_eq!(build.id().len(), 16);
}

#[tokio::test]
async fn should_send_an_added_resource_of_a_build() {
    let (client, mut rx) = connected(Arc::new(build_handler), None).await;
    let build = create_build(&client).await;

    build.add_resource("lib/a.src", "print 1").await.unwrap();

    let requests = received(&mut rx, 2).await;

    client.shutdown().await;

    let MessageHookNotification::AddResourceToBuildRpc(added) = &requests[1] else {
        panic!("expected AddResourceToBuild");
    };

    assert_eq!(
        (&*added.location, added.zip_content.text().as_str()),
        ("lib/a.src", "print 1")
    );
}

#[tokio::test]
async fn should_finish_a_performed_build() {
    let (client, _rx) = connected(Arc::new(build_handler), None).await;
    let build = prepared_build(&client).await;
    let outcome = build
        .perform_and_wait(Some(Duration::from_secs(1)))
        .await
        .unwrap();

    client.shutdown().await;
    assert!(matches!(&outcome, BuildOutcome::Completion(_)));
    assert_eq!(outcome.state(), BuildState::Complete);
    assert_eq!(outcome.output(), "built");
}

#[tokio::test]
async fn should_dispose_a_finished_build() {
    let (client, _rx) = connected(Arc::new(build_handler), None).await;
    let build = prepared_build(&client).await;

    build
        .perform_and_wait(Some(Duration::from_secs(1)))
        .await
        .unwrap();

    let disposed = build.dispose().await.unwrap();

    client.shutdown().await;
    assert!(disposed.dispose_success);
}

#[tokio::test]
async fn should_create_a_context_with_its_program_and_params() {
    let (client, mut rx) = connected(Arc::new(context_handler), None).await;
    let context = client.create_context(context_options()).await.unwrap();
    let requests = received(&mut rx, 1).await;

    client.shutdown().await;
    assert_eq!(context.terminal_pid(), PID);

    let MessageHookNotification::CreateContextRpc(create) = &requests[0] else {
        panic!("expected CreateContextRpc")
    };

    assert_eq!(create.program_name, "myprogram");

    let params: Vec<String> = create.params.json().unwrap();

    assert_eq!(params, ["a"]);
}

#[tokio::test]
async fn should_deliver_the_events_of_a_context() {
    let (client, context, _rx) = connected_context(Arc::new(context_handler)).await;

    let ids = run_line(&context).await;

    client.shutdown().await;

    assert_eq!(ids, CONTEXT_STEP_EVENTS);
}

#[tokio::test]
async fn should_deliver_what_a_context_prints_before_its_events_are_subscribed_to() {
    let (client, context, _rx) = connected_context(Arc::new(eager_context_handler)).await;

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut events = context.events();

    let first = tokio::time::timeout(Duration::from_secs(5), events.next())
        .await
        .unwrap();

    client.shutdown().await;

    let Some(GreybelNotification::PrintSentClientRpc(print)) = first else {
        panic!("expected what the context printed, got {first:?}");
    };

    assert_eq!(print.zip_output.text(), "at once");
}

#[tokio::test]
async fn should_dispose_a_context() {
    let (client, context, mut rx) = connected_context(Arc::new(context_handler)).await;

    run_line(&context).await;

    context.dispose().await.unwrap();

    let requests = received(&mut rx, 3).await;

    client.shutdown().await;

    assert!(matches!(
        requests[2],
        MessageHookNotification::DisposeContextRpc(_)
    ));
}

#[tokio::test]
async fn should_time_out_a_request_without_a_response() {
    let (client, _rx) = connected(Arc::new(|_| vec![]), None).await;
    let error = client.health_check().await.unwrap_err();
    let connected = client.is_connected();

    client.shutdown().await;
    assert!(matches!(error, ClientError::Timeout(_)), "{error}");
    assert!(connected);
}

#[tokio::test]
async fn should_end_requests_and_subscriptions_when_the_server_disconnects() {
    let (client, _rx) = connected(Arc::new(|_| vec![]), Some(1)).await;
    let mut subscription = client.subscribe(|_| true);
    let error = client.health_check().await.unwrap_err();

    assert!(matches!(error, ClientError::Disconnect), "{error}");
    assert_eq!(subscription.next().await, None);
    assert!(!client.is_connected());
}

#[tokio::test]
async fn should_fail_requests_after_the_server_disconnected() {
    let (client, _rx) = connected(Arc::new(|_| vec![]), Some(1)).await;
    let mut subscription = client.subscribe(|_| true);
    let _ = client.health_check().await;
    let _ = subscription.next().await;
    let result = client.health_check().await;

    assert!(matches!(result, Err(ClientError::Disconnect)));
}

#[tokio::test]
async fn should_end_subscriptions_on_shutdown() {
    let (client, _rx) = connected(Arc::new(|_| vec![status()]), None).await;
    let mut subscription = client.subscribe(|_| true);

    client.shutdown().await;
    assert_eq!(subscription.next().await, None);
}

#[tokio::test]
async fn should_fail_requests_after_shutdown() {
    let (client, _rx) = connected(Arc::new(|_| vec![status()]), None).await;
    let mut subscription = client.subscribe(|_| true);

    client.shutdown().await;

    let _ = subscription.next().await;
    let result = client.health_check().await;

    assert!(matches!(result, Err(ClientError::Disconnect)));
}
