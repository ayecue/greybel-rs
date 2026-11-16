//! Shared editor/runtime harnesses and fixtures for integration tests.

#![allow(dead_code)]

pub mod editor;
pub mod game;
pub mod host;
pub mod runtime;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::Poll;

use duplex_core::{Channel, ChannelOptions, Exchange, Message};
use duplex_test_support::future::poll;
use duplex_test_support::{Memory, MemoryQueue};
use editor::{
    Editor, EditorRouter, Exit, Launch, LaunchFailure, Scopes, Step, StepMode, StopNotice,
    StopReason, Variable, Variables,
};
use greybel_documents_host_provider::protocol::Editor as DocumentsEditor;
use greybel_io_web::protocol::Page;
use greybel_runtime::{EditorHost, Runtime};
use greybel_test_support::file_editor::FileEditor;
use greybel_test_support::game::RUN_TIME;
use greybel_test_support::terminal::TerminalRecorder;

/// Duplex response future returned when the editor sends message `M`.
type Pending<M> = <<M as Message>::Exchange as Exchange>::Handle;

/// Response output type produced by message `M`.
type Response<M> = <Pending<M> as Future>::Output;

pub const MAIN: &str = "file:///project/main.src";
pub const LIBRARY: &str = "file:///project/lib.src";

/// Recorded logpoint message and its optional source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log {
    pub text: String,
    pub line: Option<u32>,
}

/// Recorded stop reason and its optional source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub reason: StopReason,
    pub line: Option<u32>,
}

/// Memory-connected editor and runtime with editor-hosted documents and input, plus captured terminal output,
/// stops, and logpoints.
pub struct Harness {
    queue: MemoryQueue,
    pub editor: Channel<Editor, Memory>,
    pub stops: Rc<RefCell<Vec<Stop>>>,
    /// Captured logpoint messages.
    pub logs: Rc<RefCell<Vec<Log>>>,
    /// Captured program terminal.
    pub terminal: Rc<TerminalRecorder>,
    serve: Pin<Box<dyn Future<Output = ()>>>,
    _page: Channel<Page, Memory>,
    _documents: Channel<DocumentsEditor, Memory>,
}

impl Harness {
    pub fn new(files: &[(&str, &str)]) -> Self {
        Self::with_runtime(files, |runtime| runtime)
    }

    pub fn with_runtime(
        files: &[(&str, &str)],
        configure: impl FnOnce(Runtime<EditorHost<Memory>>) -> Runtime<EditorHost<Memory>>,
    ) -> Self {
        let (editor_transport, runtime_transport, queue) = Memory::transports();
        let serve = Box::pin(configure(Runtime::new(&runtime_transport)).serve());
        let editor = Channel::new(&editor_transport, ChannelOptions::default());
        let page = Channel::new(&editor_transport, ChannelOptions::default());
        let documents = Channel::new(&editor_transport, ChannelOptions::default());
        let stops = Rc::default();
        let logs = Rc::default();

        Stops {
            stops: Rc::clone(&stops),
            logs: Rc::clone(&logs),
        }
        .attach(&editor);

        let recorder = TerminalRecorder::serve(&page);

        FileEditor::serve(&documents, files);

        let mut harness = Self {
            queue,
            editor,
            stops,
            logs,
            terminal: recorder,
            serve,
            _page: page,
            _documents: documents,
        };

        harness.settle();

        harness
    }

    /// Polls the runtime and delivers memory batches until two consecutive rounds make no progress, up to 64 rounds.
    fn settle(&mut self) {
        for _ in 0..64 {
            self.poll_serve();
            if self.queue.deliver() == 0 {
                self.poll_serve();
                if self.queue.deliver() == 0 {
                    return;
                }
            }
        }
    }

    /// Drives `future` while delivering memory batches and polling the runtime until it is ready.
    pub fn wait<F: Future>(&mut self, future: F) -> F::Output {
        let serve = &mut self.serve;

        self.queue.drive_with(future, || {
            let _ = poll(serve);
        })
    }

    /// Like [`Harness::wait`], advancing the transport clock by `step` on every polling round.
    pub fn wait_advancing<F: Future>(&mut self, future: F, step: std::time::Duration) -> F::Output {
        let serve = &mut self.serve;
        let queue = &self.queue;

        queue.drive_with(future, || {
            let _ = poll(serve);

            queue.advance(step);
        })
    }

    /// Advances the transport's manual clock by `by`.
    pub fn advance(&self, by: std::time::Duration) {
        self.queue.advance(by);
    }

    /// Drives `future` with real-I/O polling, sleeping 5 ms between rounds until ready. Panics after `limit`.
    pub async fn wait_on_io<F: Future>(
        &mut self,
        future: F,
        limit: std::time::Duration,
    ) -> F::Output {
        let mut future = pin!(future);
        let deadline = std::time::Instant::now() + limit;

        while std::time::Instant::now() < deadline {
            if let Poll::Ready(output) = poll(&mut future) {
                return output;
            }

            self.poll_serve();
            self.queue.deliver();
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        panic!("nothing happened within {limit:?}");
    }

    /// Sends an editor request and drives the harness until its response arrives.
    pub fn request<M>(&mut self, message: &M) -> Response<M>
    where
        M: Message<Sender = Editor>,
        Pending<M>: Future,
    {
        let pending = self.editor.send(message).expect("sent");

        self.wait(pending)
    }

    /// Sends a request and waits for its response using real-I/O polling, up to [`RUN_TIME`].
    pub async fn request_on_io<M>(&mut self, message: &M) -> Response<M>
    where
        M: Message<Sender = Editor>,
        Pending<M>: Future,
    {
        let pending = self.editor.send(message).expect("sent");

        self.wait_on_io(pending, RUN_TIME).await
    }

    /// Sends a debugger step notification with `mode`.
    pub fn step(&self, mode: StepMode) {
        self.editor.send(&Step { mode }).expect("sent");
    }

    /// Returns variables from the globals scope of `frame`.
    pub fn globals(&mut self, frame: u32) -> Vec<Variable> {
        let scopes = self.request(&Scopes { frame }).expect("scopes");

        self.request(&Variables {
            reference: scopes[1].reference,
        })
        .expect("globals")
    }

    /// Uses real-I/O polling until the editor has recorded `count` stops.
    pub async fn stops_on_io(&mut self, count: usize, limit: std::time::Duration) {
        let stopped = self.stopped(count);

        self.wait_on_io(stopped, limit).await;
    }

    /// Drives the memory transport until the editor has recorded `count` stops.
    pub fn wait_for_stops(&mut self, count: usize) {
        let stopped = self.stopped(count);

        self.wait(stopped);
    }

    /// Returns a future that resolves once the editor has recorded `count` stops.
    fn stopped(&self, count: usize) -> impl Future<Output = ()> + use<> {
        let stops = Rc::clone(&self.stops);

        std::future::poll_fn(move |_| {
            if stops.borrow().len() >= count {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
    }

    /// Polls the runtime and delivers memory batches for `rounds` iterations.
    pub fn run_for(&mut self, rounds: usize) {
        for _ in 0..rounds {
            self.poll_serve();
            self.queue.deliver();
        }
    }

    fn poll_serve(&mut self) {
        let _ = poll(&mut self.serve);
    }

    pub fn launch(
        &mut self,
        debug: bool,
        stop_on_entry: bool,
    ) -> impl Future<Output = Result<Exit, duplex_core::Error<LaunchFailure>>> + use<> {
        self.launch_with(debug, stop_on_entry, &[])
    }

    pub fn launch_in_game(
        &mut self,
        port: u16,
        debug: bool,
    ) -> impl Future<Output = Result<Exit, duplex_core::Error<LaunchFailure>>> + use<> {
        self.editor
            .send(&Launch {
                entry: MAIN.into(),
                root: None,
                extensions: extensions(),
                environment: editor::Environment::Game { port },
                debug,
                stop_on_entry: false,
                params: Vec::new().into(),
                variables: BTreeMap::new(),
                strict_mode: false,
                program_name: "runtime".into(),
                disabled_syntax: None,
            })
            .expect("the launch is sent")
    }

    pub fn launch_with(
        &mut self,
        debug: bool,
        stop_on_entry: bool,
        variables: &[(&str, &str)],
    ) -> impl Future<Output = Result<Exit, duplex_core::Error<LaunchFailure>>> + use<> {
        let variables = variables
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();

        self.send_launch(&Launch {
            variables,
            ..mock_launch(debug, stop_on_entry)
        })
    }

    pub fn send_launch(
        &mut self,
        launch: &Launch<'static>,
    ) -> impl Future<Output = Result<Exit, duplex_core::Error<LaunchFailure>>> + use<> {
        self.editor.send(launch).expect("the launch is sent")
    }

    pub fn last_stop(&self) -> Stop {
        self.stops.borrow().last().cloned().expect("a stop")
    }

    /// Returns recorded stop lines in notification order.
    pub fn stop_lines(&self) -> Vec<Option<u32>> {
        self.stops.borrow().iter().map(|stop| stop.line).collect()
    }
}

/// Finds the variable named `name`, panicking with the available variables if absent.
pub fn named<'v>(variables: &'v [Variable], name: &str) -> &'v Variable {
    variables
        .iter()
        .find(|variable| variable.name == name)
        .unwrap_or_else(|| panic!("no {name} in {variables:?}"))
}

struct Stops {
    stops: Rc<RefCell<Vec<Stop>>>,
    logs: Rc<RefCell<Vec<Log>>>,
}

impl EditorRouter for Stops {
    fn on_stop_notice(
        &self,
        message: editor::StopNoticeRef<'_>,
        _next: duplex_core::NextNotification<StopNotice>,
    ) {
        self.stops.borrow_mut().push(Stop {
            reason: message.reason,
            line: message.location.as_ref().map(|location| location.line),
        });
    }

    fn on_log_message(
        &self,
        message: editor::LogMessageRef<'_>,
        _next: duplex_core::NextNotification<editor::LogMessage<'static>>,
    ) {
        self.logs.borrow_mut().push(Log {
            text: message.text.to_string(),
            line: message.location.as_ref().map(|location| location.line),
        });
    }
}

/// Creates the default mock-world launch for `main.src` with seed `test` and the requested debug options.
pub fn mock_launch(debug: bool, stop_on_entry: bool) -> Launch<'static> {
    Launch {
        entry: MAIN.into(),
        root: None,
        extensions: extensions(),
        environment: editor::Environment::Mock {
            seed: "test".into(),
            memory_limit: 1024,
        },
        debug,
        stop_on_entry,
        params: Vec::new().into(),
        variables: BTreeMap::new(),
        strict_mode: false,
        program_name: "myprogram".into(),
        disabled_syntax: None,
    }
}

/// Returns the editor's default import extensions: `src`, `gs`, and `ms`.
pub fn extensions() -> std::borrow::Cow<'static, [String]> {
    ["src", "gs", "ms"].map(str::to_owned).to_vec().into()
}
