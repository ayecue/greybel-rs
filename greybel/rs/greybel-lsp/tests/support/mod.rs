//! Shared client and fixture helpers for `greybel-lsp` integration tests.

#![allow(dead_code)]

pub mod highlight;
pub mod project;

use std::collections::{BTreeMap, HashMap, VecDeque};

use greybel_lsp::types::{self as lsp, Notification as _, Request as _};
use greybel_lsp::{
    FindFilesRequest, InitializationOptions, Message, Notification, ReadFileRequest, Request,
    RequestId, Response, Server,
};
use greybel_test_support::lsp::{did_open_params, initialize_params};
use serde_json::{Value, json};

pub const ROOT: &str = "file:///My%20Project%23%C3%A9";

pub fn uri(path: &str) -> String {
    format!("{ROOT}/{path}")
}

pub struct Client {
    pub server: Server,
    pub files: BTreeMap<String, String>,
    pub settings: Value,
    pub options: InitializationOptions,
    pub responds_to_reads: bool,
    /// Read-file requests queued while [`Client::responds_to_reads`] is false.
    pub pending_requests: Vec<Request>,
    pub requests: Vec<String>,
    pub diagnostics: HashMap<String, Vec<lsp::Diagnostic>>,
    pub published: Vec<String>,
    responses: HashMap<RequestId, Response>,
    next_id: i64,
}

impl Client {
    pub fn new(files: &[(&str, &str)]) -> Self {
        Self {
            server: Server::new(),
            files: files
                .iter()
                .map(|(path, text)| (uri(path), (*text).to_owned()))
                .collect(),
            settings: json!({}),
            options: InitializationOptions::default(),
            responds_to_reads: true,
            pending_requests: Vec::new(),
            requests: Vec::new(),
            diagnostics: HashMap::new(),
            published: Vec::new(),
            responses: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn started(files: &[(&str, &str)]) -> Self {
        let mut client = Self::new(files);

        client.initialize();

        client
    }

    /// Creates an initialized client serving `files` with `settings`.
    pub fn with_settings(files: &[(&str, &str)], settings: Value) -> Self {
        let mut client = Self::new(files);

        client.settings = settings;
        client.initialize();

        client
    }

    /// Creates an initialized client serving `files` with the server's default dialect set to `dialect`.
    pub fn in_dialect(files: &[(&str, &str)], dialect: &str) -> Self {
        let mut client = Self::new(files);

        client.options.dialect = Some(dialect.to_owned());
        client.initialize();

        client
    }

    pub fn initialize(&mut self) -> lsp::InitializeResult {
        let result = self.try_initialize().expect("initializes");

        serde_json::from_value(result).expect("an initialize result")
    }

    /// Sends `initialize` with the client's options and, on success, sends the `initialized` notification.
    pub fn try_initialize(&mut self) -> Result<Value, greybel_lsp::ResponseError> {
        let mut params = initialize_params(
            ROOT,
            json!({
                "workspace": { "configuration": true },
                "textDocument": { "hover": { "contentFormat": ["markdown", "plaintext"] } }
            }),
        );

        params["initializationOptions"] = json!(self.options);

        let result = self.request_value(lsp::InitializeRequest::METHOD.as_str(), params)?;

        self.notify_value(lsp::InitializedNotification::METHOD.as_str(), json!({}));

        Ok(result)
    }

    pub fn send(&mut self, message: Message) {
        let mut queue = VecDeque::from([message]);

        while let Some(message) = queue.pop_front() {
            let outgoing = self.server.receive(message);

            for message in outgoing {
                if let Some(reply) = self.handle(message) {
                    queue.push_back(reply);
                }
            }
        }
    }

    pub fn release_reads(&mut self) {
        let reads = std::mem::take(&mut self.pending_requests);

        for request in reads {
            let reply = self.read(&request);

            self.send(reply);
        }
    }

    fn handle(&mut self, message: Message) -> Option<Message> {
        match message {
            Message::Response(response) => {
                self.responses
                    .insert(response.id.clone().expect("an id"), response);

                None
            }
            Message::Notification(notification) => {
                if notification.method == lsp::PublishDiagnosticsNotification::METHOD.as_str() {
                    let params: lsp::PublishDiagnosticsParams =
                        serde_json::from_value(notification.params).expect("diagnostics");

                    self.published.push(params.uri.0.clone());
                    self.diagnostics.insert(params.uri.0, params.diagnostics);
                }

                None
            }
            Message::Request(request) => {
                self.requests.push(request.method.clone());

                let method = request.method.as_str();

                if method == ReadFileRequest::METHOD.as_str() {
                    if !self.responds_to_reads {
                        self.pending_requests.push(request);

                        return None;
                    }

                    return Some(self.read(&request));
                }

                let result = if method == FindFilesRequest::METHOD.as_str() {
                    let params = request
                        .params::<FindFilesRequest>()
                        .expect("find-files parameters");

                    assert!(params.include.starts_with("**/*."), "{}", params.include);

                    json!(self.files.keys().collect::<Vec<_>>())
                } else if method == lsp::ConfigurationRequest::METHOD.as_str() {
                    json!([self.settings.clone()])
                } else {
                    Value::Null
                };

                Some(Message::Response(Response {
                    id: Some(request.id),
                    outcome: Ok(result),
                }))
            }
        }
    }

    fn read(&self, request: &Request) -> Message {
        let params = request
            .params::<ReadFileRequest>()
            .expect("read-file parameters");

        Message::Response(Response {
            id: Some(request.id.clone()),
            outcome: Ok(json!(self.files.get(&params.uri))),
        })
    }

    pub fn request_value(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, greybel_lsp::ResponseError> {
        let id = RequestId::Number(self.next_id);

        self.next_id += 1;
        self.send(Message::Request(Request {
            id: id.clone(),
            method: method.to_owned(),
            params,
        }));

        self.responses.remove(&id).expect("a response").outcome
    }

    pub fn request<R: lsp::Request>(&mut self, params: Value) -> R::Result {
        let value = self
            .request_value(R::METHOD.as_str(), params)
            .unwrap_or_else(|error| panic!("{} failed: {error:?}", R::METHOD.as_str()));

        serde_json::from_value(value).expect("a result")
    }

    pub fn notify_value(&mut self, method: &str, params: Value) {
        self.send(Message::Notification(Notification {
            method: method.to_owned(),
            params,
        }));
    }

    pub fn open(&mut self, path: &str, text: &str) {
        self.open_as(path, text, "greyscript");
    }

    pub fn open_as(&mut self, path: &str, text: &str, language: &str) {
        self.notify_value(
            lsp::DidOpenTextDocumentNotification::METHOD.as_str(),
            did_open_params(&uri(path), language, text),
        );
    }

    pub fn change(&mut self, path: &str, version: i32, changes: Value) {
        self.notify_value(
            lsp::DidChangeTextDocumentNotification::METHOD.as_str(),
            json!({ "textDocument": { "uri": uri(path), "version": version }, "contentChanges": changes }),
        );
    }

    pub fn close(&mut self, path: &str) {
        self.notify_value(
            lsp::DidCloseTextDocumentNotification::METHOD.as_str(),
            json!({ "textDocument": { "uri": uri(path) } }),
        );
    }

    pub fn codes(&self, path: &str) -> Vec<String> {
        self.diagnostics
            .get(&uri(path))
            .map(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| match &diagnostic.code {
                        Some(lsp::Code::String(code)) => code.clone(),
                        other => format!("{other:?}"),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn position(path: &str, line: u32, character: u32) -> Value {
        json!({ "textDocument": { "uri": uri(path) }, "position": { "line": line, "character": character } })
    }

    /// Requests and returns the flat document-symbol list for `path`.
    pub fn document_symbols(&mut self, path: &str) -> Vec<lsp::DocumentSymbol> {
        let symbols = self
            .request::<lsp::DocumentSymbolRequest>(json!({ "textDocument": { "uri": uri(path) } }));
        let Some(lsp::DocumentSymbolResponse::DocumentSymbolList(symbols)) = symbols else {
            panic!("symbols");
        };

        symbols
    }

    /// Returns the completion item named `label` at `line` and `character` of `path`.
    pub fn completion(
        &mut self,
        path: &str,
        line: u32,
        character: u32,
        label: &str,
    ) -> lsp::CompletionItem {
        let items = self.request::<lsp::CompletionRequest>(Self::position(path, line, character));
        let Some(lsp::CompletionResponse::CompletionItemList(items)) = items else {
            panic!("items");
        };

        items
            .into_iter()
            .find(|item| item.label == label)
            .unwrap_or_else(|| panic!("no completion {label}"))
    }
}

/// Creates an LSP `Position` at `line` and `character`.
pub fn lsp_position(line: u32, character: u32) -> lsp::Position {
    lsp::Position { line, character }
}

/// Creates a single zero-length content change inserting `text` at `line` and `character`.
pub fn insertion(line: u32, character: u32, text: &str) -> Value {
    json!([{
        "range": {
            "start": { "line": line, "character": character },
            "end": { "line": line, "character": character }
        },
        "text": text
    }])
}

pub const LIBRARY: &str = "// Says hello.
// @param {string} name
// @returns {string}
greet = function(name)
  return \"hello \" + name
end function
";

pub const MAIN: &str = "#include \"lib/greet.src\"
message = greet(\"you\")
print message
";

pub fn project() -> Client {
    Client::started(&[("lib/greet.src", LIBRARY), ("main.src", MAIN)])
}

/// Creates an uninitialized client and records its refused hover request.
pub fn client_refused_before_initialize() -> Client {
    let mut client = Client::new(&[]);
    let _ = client.request_value(
        lsp::HoverRequest::METHOD.as_str(),
        Client::position("main.src", 0, 0),
    );

    client
}

/// Initializes the client from [`client_refused_before_initialize`], then requests an unknown method.
pub fn client_asked_an_unknown_method() -> Client {
    let mut client = client_refused_before_initialize();

    client.initialize();

    let _ = client.request_value("textDocument/unknown", json!({}));

    client
}

/// Shuts down the client from [`client_asked_an_unknown_method`].
pub fn client_shut_down() -> Client {
    let mut client = client_asked_an_unknown_method();

    client
        .request_value(lsp::ShutdownRequest::METHOD.as_str(), Value::Null)
        .expect("shuts down");

    client
}

/// Opens `main.src` in [`project`] with `print missing`.
pub fn project_with_a_missing_reference() -> Client {
    let mut client = project();

    client.open("main.src", "print missing\n");

    client
}

/// Edits the project from [`project_with_a_missing_reference`] to define `missing`.
pub fn project_with_the_reference_defined() -> Client {
    let mut client = project_with_a_missing_reference();

    client.change("main.src", 2, insertion(0, 0, "missing = 1\n"));

    client
}

/// Makes a whitespace edit in [`project_with_the_reference_defined`] without changing its diagnostics.
pub fn project_edited_without_new_diagnostics() -> Client {
    let mut client = project_with_the_reference_defined();

    client.change("main.src", 3, insertion(1, 13, " "));

    client
}

/// Opens `main.src` in [`project`] with an emoji in its source.
pub fn project_with_an_emoji() -> Client {
    let mut client = project();

    client.open("main.src", "s = \"😀\"\nprint s\n");

    client
}

/// Inserts `!` immediately after the emoji in [`project_with_an_emoji`].
pub fn change_after_the_emoji(client: &mut Client) {
    client.change("main.src", 2, insertion(0, 7, "!"));
}

/// Creates an initialized client serving the library with read-file responses disabled.
pub fn client_deferring_reads() -> Client {
    let mut client = Client::new(&[("lib/greet.src", LIBRARY)]);

    client.responds_to_reads = false;
    client.initialize();

    client
}

/// Edits `main.src` while read-file responses are deferred in [`client_deferring_reads`], then enables responses.
pub fn client_edited_while_reading() -> Client {
    let mut client = client_deferring_reads();

    client.open("main.src", MAIN);

    let edited = format!("{MAIN}print greet(1)\n");

    client.change("main.src", 2, json!([{ "text": edited }]));
    client.responds_to_reads = true;

    client
}

/// Creates an initialized client without library files and opens `main.src`.
pub fn client_missing_the_library() -> Client {
    let mut client = Client::started(&[]);

    client.open("main.src", MAIN);

    client
}

/// Opens `main.src` in [`project`] with an include and an incomplete `g` identifier.
pub fn project_completing_greet() -> Client {
    let mut client = project();

    client.open("main.src", "#include \"lib/greet.src\"\ng\n");

    client
}

/// Creates a project containing the library, `main.src`, and `tool.src`, with the library open for renaming.
pub fn project_renaming_greet() -> Client {
    let mut client = Client::started(&[
        ("lib/greet.src", LIBRARY),
        ("main.src", MAIN),
        (
            "tool.src",
            "#include \"lib/greet.src\"\nprint greet(\"x\")\n",
        ),
    ]);

    client.open("lib/greet.src", LIBRARY);

    client
}

/// Builds rename parameters for changing `greet` in the library to `new_name`.
pub fn rename_greet(new_name: &str) -> Value {
    json!({
        "textDocument": { "uri": uri("lib/greet.src") },
        "position": { "line": 3, "character": 1 },
        "newName": new_name
    })
}

/// Opens `main.src` in [`project`].
pub fn project_with_main_open() -> Client {
    let mut client = project();

    client.open("main.src", MAIN);

    client
}

/// Creates an initialized client with hover documentation and diagnostics disabled.
pub fn client_with_features_off() -> Client {
    Client::with_settings(
        &[],
        json!({ "hoverdocs": false, "diagnostic": false, "fileExtensions": "src" }),
    )
}
