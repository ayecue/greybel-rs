//! Shared helpers for `greybel-lsp-stdio` integration tests.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use std::{fs, thread};

use greybel_lsp::Uri;
use greybel_test_support::TempFolder;
use greybel_test_support::lsp::{did_open_params, initialize_params};
use serde_json::{Value, json};

/// On-disk test project deleted on drop.
pub struct Project(TempFolder);

impl Project {
    pub fn new(files: &[(&str, &str)]) -> Self {
        Self(TempFolder::new("greybel-lsp-stdio", files.iter().copied()))
    }

    pub fn root(&self) -> &Path {
        self.0.path()
    }

    pub fn uri(&self, path: &str) -> String {
        Uri::file(&self.root().join(path).to_string_lossy()).to_string()
    }
}

pub fn send(stdin: &mut ChildStdin, message: &Value) {
    let body = serde_json::to_vec(message).expect("serializes");

    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("writes");
    stdin.write_all(&body).expect("writes");
    stdin.flush().expect("flushes");
}

pub fn messages(stdout: impl Read + Send + 'static) -> Receiver<Value> {
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);

        loop {
            let mut length = 0;
            let mut line = String::new();

            loop {
                line.clear();
                if reader.read_line(&mut line).expect("reads") == 0 {
                    return;
                }

                let header = line.trim_end();

                if header.is_empty() {
                    break;
                }

                if let Some(value) = header.strip_prefix("Content-Length: ") {
                    length = value.parse().expect("a length");
                }
            }

            let mut body = vec![0; length];

            reader.read_exact(&mut body).expect("reads the body");
            if sender
                .send(serde_json::from_slice(&body).expect("json"))
                .is_err()
            {
                return;
            }
        }
    });

    receiver
}

pub fn next(receiver: &Receiver<Value>, mut wanted: impl FnMut(&Value) -> bool) -> Value {
    loop {
        let message = receiver
            .recv_timeout(Duration::from_secs(120))
            .expect("the server answers in time");

        if wanted(&message) {
            return message;
        }
    }
}

/// LSP server process running against an on-disk project with a library and a `main.src` that includes it.
pub struct Session {
    pub project: Project,
    pub child: Child,
    pub stdin: ChildStdin,
    pub receiver: Receiver<Value>,
    /// Method names of requests and notifications sent by the server.
    pub methods: Vec<String>,
}

impl Session {
    /// Starts the server `binary` (the test's `CARGO_BIN_EXE_greybel-lsp`) with its default project and pipes.
    pub fn start(binary: &str) -> Self {
        Self::start_with(binary, &[])
    }

    pub fn start_with(binary: &str, args: &[&str]) -> Self {
        let project = Project::new(&[
            (
                "lib/greet.src",
                "greet = function(name)\n  return \"hi \" + name\nend function\n",
            ),
            (
                "main.src",
                "#include \"lib/greet\"\nprint greet(\"x\")\nprint nope\n",
            ),
            ("node_modules/dep/vendored.src", "vendored = 1\n"),
            (".config/hidden.src", "hidden = 1\n"),
        ]);
        let mut child = Command::new(binary)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("starts");
        let stdin = child.stdin.take().expect("stdin");
        let receiver = messages(child.stdout.take().expect("stdout"));

        Self {
            project,
            child,
            stdin,
            receiver,
            methods: Vec::new(),
        }
    }

    /// Sends request `id` calling `method` with optional `params`, then waits for and returns its response.
    fn request(&mut self, id: u32, method: &str, params: Option<Value>) -> Value {
        let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });

        if let Some(params) = params {
            message["params"] = params;
        }

        send(&mut self.stdin, &message);

        self.next(|message| message["id"] == id)
    }

    fn next(&mut self, wanted: impl Fn(&Value) -> bool) -> Value {
        let methods = &mut self.methods;

        next(&self.receiver, |message| {
            if let Some(method) = message["method"].as_str() {
                methods.push(method.to_owned());
            }

            wanted(message)
        })
    }

    /// Sends the notification `method` with optional `params`.
    fn notify(&mut self, method: &str, params: Option<Value>) {
        let mut message = json!({ "jsonrpc": "2.0", "method": method });

        if let Some(params) = params {
            message["params"] = params;
        }

        send(&mut self.stdin, &message);
    }

    pub fn initialize(&mut self) -> Value {
        let params = initialize_params(&self.project.uri(""), json!({}));

        self.request(1, "initialize", Some(params))
    }

    pub fn initialized(&mut self) {
        self.notify("initialized", Some(json!({})));
    }

    /// Opens `main.src` and waits for its `publishDiagnostics` message.
    pub fn open_main(&mut self) -> Value {
        let main = self.project.uri("main.src");
        let text = fs::read_to_string(self.project.root().join("main.src")).expect("reads");

        self.notify(
            "textDocument/didOpen",
            Some(did_open_params(&main, "greyscript", &text)),
        );

        self.next(|message| {
            message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == main
        })
    }

    /// Sends `workspace/didChangeConfiguration` with the server's `greybel` settings.
    pub fn configure(&mut self, settings: Value) {
        self.notify(
            "workspace/didChangeConfiguration",
            Some(json!({ "settings": { "greybel": settings } })),
        );
    }

    pub fn workspace_symbols(&mut self, query: &str) -> Value {
        self.request(2, "workspace/symbol", Some(json!({ "query": query })))
    }

    pub fn shutdown(&mut self) {
        self.request(3, "shutdown", None);
    }

    pub fn exit(&mut self) -> ExitStatus {
        self.notify("exit", None);

        self.child.wait().expect("exits")
    }
}

/// Starts a session and completes the `initialize` request and `initialized` notification.
pub fn initialized_session(binary: &str) -> Session {
    let mut session = Session::start(binary);

    session.initialize();
    session.initialized();

    session
}

/// Initializes a session and opens `main.src`.
pub fn session_with_main_open(binary: &str) -> Session {
    let mut session = initialized_session(binary);

    session.open_main();

    session
}

/// Requests workspace symbols in [`session_with_main_open`], then shuts down the session.
pub fn shut_down_session(binary: &str) -> Session {
    let mut session = session_with_main_open(binary);

    session.workspace_symbols("greet");
    session.shutdown();

    session
}

pub fn codes(published: &Value) -> Vec<&str> {
    published["params"]["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .map(|diagnostic| diagnostic["code"].as_str().expect("a code"))
        .collect()
}
