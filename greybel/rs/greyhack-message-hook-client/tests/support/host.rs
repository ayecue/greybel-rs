//! Runs `greyhack-test-support proxy` with the test acting as the game.

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use duplex_core::{FieldAccess, Message, Metadata, Tag, ValueRef};
use greybel_test_support::game::{port, start_tool};
use greyhack_message_hook_client::protocol::provider::{HealthCheckRpc, MessageHook};
use greyhack_message_hook_client::{Client, ClientOptions};
use serde_json::{Map, Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use super::{Incoming, Outgoing, options, status_rpc};

/// Callback mapping each received game JSON message to replies sent to every connected client.
pub type GameHandler = Arc<dyn Fn(&Value) -> Vec<Value> + Send + Sync>;

/// Running proxy. Client game messages are forwarded to the handler passed to [`Host::connected`], whose replies are
/// broadcast to every client.
pub struct Host {
    child: Child,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    received: Mutex<Option<Receiver<Value>>>,
    pub port: u16,
}

impl Host {
    /// Answer time a request gets: the proxy is a .NET process whose first translations of each message type are
    /// slow on busy CI machines, where 500 ms ran out.
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

    /// Starts the proxy with its tool built in `folder` (the test's `CARGO_TARGET_TMPDIR`).
    pub fn start(folder: impl AsRef<Path>) -> Self {
        let tool = start_tool(folder.as_ref(), ["proxy"]);
        let (tx, rx) = channel();
        let output = tool.output;

        std::thread::spawn(move || {
            for line in output.map_while(Result::ok) {
                let message: Value = serde_json::from_str(&line).expect("the host writes messages");

                if tx.send(message).is_err() {
                    break;
                }
            }
        });

        Self {
            stdin: Arc::new(Mutex::new(tool.stdin)),
            port: port(&tool.ready, "port"),
            child: tool.child,
            received: Mutex::new(Some(rx)),
        }
    }

    /// Starts a proxy and client, routes every received game message through `handler`, and returns the proxy,
    /// client, and received-message stream.
    pub async fn connected(
        folder: impl AsRef<Path>,
        handler: GameHandler,
    ) -> (Self, Client, UnboundedReceiver<Value>) {
        let host = Self::start(folder);
        let received = host
            .received
            .lock()
            .unwrap()
            .take()
            .expect("the host's messages");
        let stdin = Arc::clone(&host.stdin);
        let (tx, rx) = unbounded_channel();

        std::thread::spawn(move || {
            for message in received {
                for reply in handler(&message) {
                    write(&stdin, &reply);
                }

                if tx.send(message).is_err() {
                    break;
                }
            }
        });

        let client = Client::connect(ClientOptions {
            request_timeout: Self::REQUEST_TIMEOUT,
            ..options(host.port)
        })
        .await
        .expect("the client connects");

        (host, client, rx)
    }
}

fn write(stdin: &Mutex<Option<ChildStdin>>, message: &Value) {
    let mut stdin = stdin.lock().unwrap();
    let Some(stdin) = stdin.as_mut() else {
        return;
    };
    let mut line = serde_json::to_vec(message).expect("a message writes");

    line.push(b'\n');
    stdin.write_all(&line).expect("the host reads its input");
    stdin.flush().expect("the host reads its input");
}

impl Drop for Host {
    fn drop(&mut self) {
        self.stdin.lock().unwrap().take();
        if self.child.try_wait().ok().flatten().is_none() {
            std::thread::sleep(Duration::from_millis(50));

            let _ = self.child.kill();
        }

        let _ = self.child.wait();
    }
}

/// Reads the numeric game ID from `game_message_id` or `plugin_message_id` metadata tags.
pub fn game_message_id(tags: &[Tag]) -> i128 {
    Tag::find(tags, "game_message_id")
        .or_else(|| Tag::find(tags, "plugin_message_id"))
        .and_then(Tag::integer)
        .expect("a message of the protocol has a game id")
}

/// Returns whether the game JSON `method` matches protocol message `T`.
pub fn is<T: Metadata>(message: &Value) -> bool {
    message["method"].as_i64().map(i128::from) == Some(game_message_id(T::TAGS))
}

/// Maps protocol field names to their positional game JSON parameters.
pub fn values<T: Metadata>(message: &Value) -> HashMap<&'static str, Value> {
    T::FIELDS
        .iter()
        .enumerate()
        .map(|(index, field)| (field.name, message["params"][index].clone()))
        .collect()
}

/// Encodes a protocol message as game JSON with its numeric method and positional parameters.
pub fn encode<T: Metadata + FieldAccess>(message: &T) -> Value {
    let mut game = Map::new();

    game.insert("method".into(), json!(game_message_id(T::TAGS)));
    game.insert(
        "params".into(),
        Value::Array(
            message
                .fields()
                .iter()
                .map(|field| json_of(&message.field(field.name).expect("a field of the message")))
                .collect(),
        ),
    );

    Value::Object(game)
}

fn json_of(value: &ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Bool(value) => json!(value),
        ValueRef::Integer(value) => json!(value),
        ValueRef::Float(value) => json!(value),
        ValueRef::Char(value) => json!(value.to_string()),
        ValueRef::Text(value) => json!(value),
        ValueRef::Bytes(value) => json!(BASE64.encode(value)),
        ValueRef::List(items) => Value::Array(items.iter().map(json_of).collect()),
        ValueRef::Record(entries) => Value::Object(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_owned(), json_of(value)))
                .collect(),
        ),
        ValueRef::Struct(fields) => Value::Object(
            fields
                .fields()
                .iter()
                .map(|field| {
                    let name = Tag::find(field.tags, "game_field_name")
                        .and_then(Tag::text)
                        .unwrap_or(field.name);
                    let value = fields.field(field.name).expect("a field of the struct");

                    (name.to_owned(), json_of(&value))
                })
                .collect(),
        ),
        ValueRef::Member { value, .. } => json!(value),
        ValueRef::Variant { name, .. } => json!(name),
    }
}

impl Incoming for Value {
    fn is<T: Metadata>(&self) -> bool {
        is::<T>(self)
    }

    fn text<T: Metadata>(&self, name: &str) -> Option<Cow<'static, str>> {
        if !is::<T>(self) {
            return None;
        }

        let value = &values::<T>(self)[name];
        let text = value
            .as_str()
            .unwrap_or_else(|| panic!("expected the text field {name}, got {value}"));

        Some(text.to_owned().into())
    }
}

impl Outgoing for Value {
    fn message<M>(message: M) -> Self
    where
        M: Message<Sender = MessageHook> + Metadata + FieldAccess + 'static,
    {
        encode(&message)
    }
}

/// Returns [`status_rpc`] encoded as game JSON for health checks, or no replies for other messages.
pub fn game_status(message: &Value) -> Vec<Value> {
    if !is::<HealthCheckRpc>(message) {
        return Vec::new();
    }

    vec![encode(&status_rpc())]
}
