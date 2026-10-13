//! In-memory MCP client for server tests.

use greybel_mcp::{Server, ServerOptions};
use rmcp::ServiceExt as _;
use serde_json::{Value, json};
use tokio::io::{
    AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, Lines, ReadHalf, WriteHalf,
};
use tokio::task::LocalSet;

/// Server options with game integration disabled.
pub const OPTIONS: ServerOptions = ServerOptions {
    allow_game: false,
    port: 7777,
};

/// Line-delimited JSON-RPC client for an in-memory MCP server.
pub struct Client {
    writer: WriteHalf<DuplexStream>,
    lines: Lines<BufReader<ReadHalf<DuplexStream>>>,
    next_id: u64,
}

impl Client {
    /// Starts a server with `options`, connects an in-memory transport, and performs the initialize handshake.
    pub async fn connect(options: ServerOptions) -> Self {
        let (client, server) = tokio::io::duplex(1 << 20);

        tokio::task::spawn_local(async move {
            let service = Server::new(options)
                .serve(tokio::io::split(server))
                .await
                .expect("the server starts");
            let _ = service.waiting().await;
        });

        let (reader, writer) = tokio::io::split(client);
        let mut client = Self {
            writer,
            lines: BufReader::new(reader).lines(),
            next_id: 1,
        };

        client
            .request(
                "initialize",
                json!({
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0" }
                }),
            )
            .await;
        client
            .send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await;

        client
    }

    async fn send(&mut self, message: &Value) {
        let mut line = message.to_string();

        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .await
            .expect("the server reads");
    }

    /// Sends `method` with `params` and waits for its matching result.
    async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;

        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await;

        loop {
            let line = self
                .lines
                .next_line()
                .await
                .expect("the server writes")
                .expect("a response");
            let message: Value = serde_json::from_str(&line).expect("JSON");

            if message["id"] == id {
                return message["result"].clone();
            }
        }
    }

    /// Calls tool `name` with `arguments` and returns its first text content and `isError` flag.
    pub async fn call(&mut self, name: &str, arguments: Value) -> (String, bool) {
        let result = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await;
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned();

        (text, result["isError"] == true)
    }

    /// Returns tool names from `tools/list`.
    pub async fn tool_names(&mut self) -> Vec<String> {
        let result = self.request("tools/list", json!({})).await;

        result["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .map(|tool| tool["name"].as_str().expect("a name").to_owned())
            .collect()
    }
}

/// Runs `future` on a current-thread Tokio runtime and `LocalSet`.
pub fn run<F: Future>(future: F) -> F::Output {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio's runtime builds");

    LocalSet::new().block_on(&runtime, future)
}
