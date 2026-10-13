//! Shared mock-plugin, proxy, and live-game fixtures for integration tests.

#![allow(dead_code)]

pub mod game;
pub mod host;

use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;

use duplex_core::{Bytes, Channel, ChannelOptions, FieldAccess, Message, Metadata, ValueRef};
use duplex_driver_stdio::StreamDriver;
use duplex_test_support::local::run_local;
use greyhack_message_hook_client::protocol::provider::{
    AddResourceToBuildRpc, AddScriptToBuildRpc, AddedResourceToBuildRpc, AddedScriptToBuildRpc,
    BuildFinishedRpc, ContextBreakpointRpc, ContextStepRpc, CreateBuildRpc, CreateContextRpc,
    CreatedBuildRpc, CreatedContextRpc, DisposeBuildRpc, DisposedBuildRpc, FinishedContextRpc,
    MessageHook, MessageHookNotification, PerformBuildRpc, PrintSentClientRpc,
    RoutersCercanosClientRpc, StatusRpc,
};
use greyhack_message_hook_client::{
    Build, BuildState, Client, ClientOptions, Context, ContextOptions, GreybelNotification,
    Payload, ScriptType, StepMode,
};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

pub const PID: i32 = 4242;

/// Closure that sends a mock-plugin message on the provider channel.
pub type Reply = Box<dyn FnOnce(&Channel<MessageHook, StreamDriver>)>;

pub type Handler = Arc<dyn Fn(&MessageHookNotification) -> Vec<Reply> + Send + Sync>;

/// Wraps `message` as a mock-plugin reply closure.
pub fn reply<M: Message<Sender = MessageHook> + 'static>(message: M) -> Reply {
    Box::new(move |channel| {
        channel.send(&message).unwrap();
    })
}

/// Starts a single-client provider on its own thread. Sends handler replies for every received message, forwards
/// received messages to the returned stream, and closes after `close_after` messages or channel closure.
pub async fn mock_server(
    handler: Handler,
    close_after: Option<usize>,
) -> (u16, UnboundedReceiver<MessageHookNotification>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();

    listener.set_nonblocking(true).unwrap();

    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = unbounded_channel();

    std::thread::spawn(move || {
        run_local(async move {
            let listener = TcpListener::from_std(listener).unwrap();
            let (socket, _) = listener.accept().await.unwrap();
            let transport = StreamDriver::tcp(socket).unwrap();
            let channel =
                Channel::<MessageHook, StreamDriver>::new(&transport, ChannelOptions::default());
            let (closing, mut closed) = unbounded_channel::<()>();
            let seen = std::rc::Rc::new(std::cell::Cell::new(0));

            MessageHookNotification::attach(&channel, {
                let channel = channel.clone();

                move |request| {
                    for reply in handler(&request) {
                        reply(&channel);
                    }

                    let _ = tx.send(request);

                    seen.set(seen.get() + 1);
                    if close_after == Some(seen.get()) {
                        let _ = closing.send(());
                    }
                }
            });

            tokio::select! {
                _ = closed.recv() => {}
                _ = channel.close_event() => {}
            }
        });
    });

    (port, rx)
}

pub fn options(port: u16) -> ClientOptions {
    ClientOptions {
        connect_attempts: 1,
        retry_delay: Duration::from_millis(10),
        request_timeout: Duration::from_millis(500),
        ..ClientOptions::with_port(port)
    }
}

/// Starts a mock server with `handler` and connects a client, returning the client and received-message stream.
pub async fn connected(
    handler: Handler,
    close_after: Option<usize>,
) -> (Client, UnboundedReceiver<MessageHookNotification>) {
    let (port, rx) = mock_server(handler, close_after).await;
    let client = Client::connect(options(port)).await.unwrap();

    (client, rx)
}

/// Connects a client to a mock server, creates a context with [`context_options`], and returns the client, context,
/// and received-message stream.
pub async fn connected_context(
    handler: Handler,
) -> (Client, Context, UnboundedReceiver<MessageHookNotification>) {
    let (client, rx) = connected(handler, None).await;
    let context = client.create_context(context_options()).await.unwrap();

    (client, context, rx)
}

/// Receives and returns exactly `count` messages from `rx`.
pub async fn received<T>(rx: &mut UnboundedReceiver<T>, count: usize) -> Vec<T> {
    let mut requests = Vec::with_capacity(count);

    for _ in 0..count {
        requests.push(rx.recv().await.unwrap());
    }

    requests
}

/// Read-side fixture abstraction over provider notifications or proxy game JSON.
pub trait Incoming {
    /// Returns whether this is protocol message `T`.
    fn is<T: Metadata>(&self) -> bool;

    /// Returns text field `name` when this is protocol message `T`, or `None` otherwise.
    fn text<T: Metadata>(&self, name: &str) -> Option<Cow<'static, str>>;
}

/// Write-side fixture abstraction creating a provider-channel [`Reply`] or proxy game JSON.
pub trait Outgoing {
    fn message<M>(message: M) -> Self
    where
        M: Message<Sender = MessageHook> + Metadata + FieldAccess + 'static;
}

impl Incoming for MessageHookNotification {
    fn is<T: Metadata>(&self) -> bool {
        self.tags() == T::TAGS
    }

    fn text<T: Metadata>(&self, name: &str) -> Option<Cow<'static, str>> {
        if !self.is::<T>() {
            return None;
        }

        match self.field(name) {
            Some(ValueRef::Text(text)) => Some(text.to_owned().into()),
            value => panic!("expected the text field {name}, got {value:?}"),
        }
    }
}

impl Outgoing for Reply {
    fn message<M>(message: M) -> Self
    where
        M: Message<Sender = MessageHook> + Metadata + FieldAccess + 'static,
    {
        reply(message)
    }
}

/// Returns a multiplayer status with a player client, plugin version 1.2.3, and game version 0.9.
pub fn status_rpc() -> StatusRpc<'static> {
    StatusRpc {
        status: true,
        has_player_client: true,
        is_single_player: false,
        plugin_version: "1.2.3".into(),
        game_version: "0.9".into(),
    }
}

pub fn status() -> Reply {
    reply(status_rpc())
}

/// Generates successful responses for build creation, resource/script addition, execution, and disposal.
pub fn build_replies<I: Incoming, O: Outgoing>(request: &I) -> Vec<O> {
    if let Some(build_id) = request.text::<CreateBuildRpc>("build_id") {
        return vec![O::message(CreatedBuildRpc { build_id })];
    }

    if let Some(build_id) = request.text::<AddResourceToBuildRpc>("build_id") {
        return vec![O::message(AddedResourceToBuildRpc { build_id })];
    }

    if let Some(build_id) = request.text::<AddScriptToBuildRpc>("build_id") {
        return vec![O::message(AddedScriptToBuildRpc { build_id })];
    }

    if let Some(build_id) = request.text::<PerformBuildRpc>("build_id") {
        return vec![O::message(BuildFinishedRpc {
            build_id,
            state: BuildState::Complete.into(),
            error_message: "".into(),
            output: Bytes::from_text("built"),
        })];
    }

    if let Some(build_id) = request.text::<DisposeBuildRpc>("build_id") {
        return vec![O::message(DisposedBuildRpc {
            build_id,
            dispose_success: true,
            state: BuildState::Complete.into(),
            error_message: "".into(),
            output: Bytes::default(),
        })];
    }

    Vec::new()
}

/// Emits an unrelated `CreatedBuild` for `other` before normal build replies, testing build-ID filtering.
pub fn build_handler(request: &MessageHookNotification) -> Vec<Reply> {
    let mut replies = Vec::new();

    if request.is::<CreateBuildRpc>() {
        replies.push(reply(CreatedBuildRpc {
            build_id: "other".into(),
        }));
    }

    replies.extend(build_replies(request));

    replies
}

pub async fn create_build(client: &Client) -> Build {
    client
        .create_build("/root/dest", "/root/res", true)
        .await
        .unwrap()
}

/// Creates a build with one resource and one post-build script.
pub async fn prepared_build(client: &Client) -> Build {
    let build = create_build(client).await;

    build.add_resource("lib/a.src", "print 1").await.unwrap();
    build.add_script(ScriptType::Post, "post").await.unwrap();

    build
}

/// Returns `CreatedContext` and a print notification in the same batch, testing buffering before `events()` is called.
pub fn eager_context_handler(request: &MessageHookNotification) -> Vec<Reply> {
    match request {
        MessageHookNotification::CreateContextRpc(r) => vec![
            reply(CreatedContextRpc {
                context_id: r.context_id.clone(),
                terminal_pid: PID,
            }),
            reply(PrintSentClientRpc {
                zip_output: Bytes::from_text("at once"),
                replace_text: false,
                window_pid: PID,
            }),
        ],
        _ => Vec::new(),
    }
}

/// Expected event names collected by [`run_line`] from [`context_replies`].
pub const CONTEXT_STEP_EVENTS: [&str; 4] = [
    "PrintSentClientRpc",
    "ContextBreakpointRpc",
    "RoutersCercanosClientRpc",
    "FinishedContextRpc",
];

/// Replies to context creation, or to a step with print, unrelated print, breakpoint, router list, and finish events.
pub fn context_replies<I: Incoming, O: Outgoing>(request: &I) -> Vec<O> {
    if let Some(context_id) = request.text::<CreateContextRpc>("context_id") {
        return vec![O::message(CreatedContextRpc {
            context_id,
            terminal_pid: PID,
        })];
    }

    let Some(context_id) = request.text::<ContextStepRpc>("context_id") else {
        return Vec::new();
    };

    vec![
        O::message(PrintSentClientRpc {
            zip_output: Bytes::from_text("hello"),
            replace_text: false,
            window_pid: PID,
        }),
        O::message(PrintSentClientRpc {
            zip_output: Bytes::from_text("not mine"),
            replace_text: false,
            window_pid: 1,
        }),
        O::message(ContextBreakpointRpc {
            context_id: context_id.clone(),
            filepath: "/a.src".into(),
            line_num: 3,
            frames: Bytes::default(),
            globals: Bytes::default(),
            breakpoint: true,
        }),
        O::message(RoutersCercanosClientRpc {
            zip_helper_routers: Bytes::from_json(&serde_json::json!([
                { "bssid": "AA", "essid": "net", "distancia": 0.1 }
            ])),
            terminal_pid: PID,
            is_lan: false,
            orig_public_ip: "".into(),
            bssid: "AA".into(),
            essid: "net".into(),
            max_acks: 9000,
        }),
        O::message(FinishedContextRpc {
            context_id,
            failed: false,
        }),
    ]
}

pub fn context_handler(request: &MessageHookNotification) -> Vec<Reply> {
    context_replies(request)
}

pub fn context_options() -> ContextOptions {
    let mut options = ContextOptions::new("print 1", "/a.src", "/");

    options.params = vec!["a".to_owned()];

    options
}

/// Steps into the next line and collects event names through context completion.
pub async fn run_line(context: &Context) -> Vec<&'static str> {
    let mut events = context.events();

    context.step(StepMode::Into).await.unwrap();

    let mut names = Vec::new();

    while let Some(notification) = events.next().await {
        names.push(notification.name());

        if matches!(notification, GreybelNotification::FinishedContextRpc(_)) {
            break;
        }
    }

    names
}
