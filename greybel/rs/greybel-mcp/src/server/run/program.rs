//! Runs a program with preloaded input and plain-text output capture.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::pin::pin;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use greybel_documents::{PathResolution, Uri};
use greybel_documents_fs_provider::FsProvider;
use greybel_io::{Echo, InputFuture, Key, PrintOptions, Stdin, Stdout, TextScreen};
use greybel_runtime::protocol::{Environment, Exit, Launch, LaunchFailure, LogMessage, StopNotice};
use greybel_runtime::{Host, Io, Runtime, connect_on_tokio, mock_world_server};
use greyhack_terminal_text::{LineLimits, LineSplitRule, plain_text};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::ServerConnection;
use scheduler::{Clock, TokioScheduler};

/// Program launch and its supplied input.
pub struct Program {
    pub launch: Launch<'static>,
    /// In-memory filesystem overrides, including directly supplied source.
    pub memory: BTreeMap<Uri, String>,
    /// Ordered input lines.
    pub input: Vec<String>,
    /// Execution timeout before forced termination.
    pub timeout: Duration,
    /// How the captured output splits long lines.
    pub split_rule: LineSplitRule,
}

/// Program exit outcome and captured terminal state.
pub struct Outcome {
    /// Terminal plain text at program exit.
    pub screen: String,
    pub exit: Result<Exit, LaunchFailure>,
    /// Indicates termination caused by timeout.
    pub out_of_time: bool,
}

impl Program {
    /// The split_rule `run.lineLimits` of `greybel.json` sets.
    pub fn split_rule_of(limits: Option<&greybel_project::LineLimits>) -> LineSplitRule {
        limits.map_or(LineSplitRule::Game, |limits| {
            LineSplitRule::Limits(LineLimits::new(
                limits.text_per_line as usize,
                limits.tags_per_line.map(|limit| limit as usize),
            ))
        })
    }

    /// Runs until exit or timeout; terminates timed-out programs and returns captured terminal output.
    /// The simulated world generates before the timeout starts, so the timeout bounds only program execution.
    pub async fn run(self) -> Outcome {
        let server = match &self.launch.environment {
            Environment::Mock { seed, .. } => {
                Some(mock_world_server(seed, Rc::new(TokioScheduler::new())).await)
            }
            Environment::Game { .. } => None,
        };
        let screen = Rc::new(RefCell::new(TextScreen::default()));
        let host = ProgramHost {
            memory: Arc::new(self.memory),
            screen: Rc::clone(&screen),
            input: Rc::new(RefCell::new(self.input.into())),
            world: server.as_ref().map(connect_on_tokio),
            split_rule: self.split_rule,
        };
        let (runtime, controller) = Runtime::with_host(host);

        tokio::task::spawn_local(runtime.serve());

        let mut exit = pin!(controller.launch(self.launch));
        let (exit, out_of_time) = match tokio::time::timeout(self.timeout, &mut exit).await {
            Ok(exit) => (exit, false),
            Err(_) => {
                controller.terminate();

                (exit.await, true)
            }
        };
        let screen = screen.borrow().text();
        drop(server);

        Outcome {
            screen,
            exit,
            out_of_time,
        }
    }
}

/// Runtime host using a memory filesystem overlay, `TextScreen` output, and preloaded input.
struct ProgramHost {
    memory: Arc<BTreeMap<Uri, String>>,
    screen: Rc<RefCell<TextScreen>>,
    input: Rc<RefCell<VecDeque<String>>>,
    world: Option<ServerConnection>,
    split_rule: LineSplitRule,
}

impl Host for ProgramHost {
    type Provider = FsProvider;

    fn provider(&self, resolution: PathResolution) -> FsProvider {
        FsProvider::new(resolution).with_memory(Arc::clone(&self.memory))
    }

    fn io(&self) -> Io {
        Io {
            stdout: Rc::new(ScreenStdout {
                screen: Rc::clone(&self.screen),
                split_rule: self.split_rule,
            }),
            stdin: Rc::new(ScreenStdin {
                screen: Rc::clone(&self.screen),
                lines: Rc::clone(&self.input),
                split_rule: self.split_rule,
            }),
        }
    }

    fn clock(&self) -> Rc<dyn Clock> {
        Rc::new(TokioScheduler::new())
    }

    fn world(&self) -> Option<ServerConnection> {
        self.world.clone()
    }

    fn connect(&self, server: &WorldServer) -> ServerConnection {
        connect_on_tokio(server)
    }

    fn report_stop(&self, _notice: &StopNotice) {}

    fn report_log(&self, _message: &LogMessage<'_>) {}
}

/// Stdout that converts rich text to plain text in a `TextScreen`.
struct ScreenStdout {
    screen: Rc<RefCell<TextScreen>>,
    split_rule: LineSplitRule,
}

impl Stdout for ScreenStdout {
    fn print(&self, text: &str, options: PrintOptions) {
        self.screen
            .borrow_mut()
            .print(&plain_text(text, self.split_rule), options);
    }

    fn clear(&self) {
        self.screen.borrow_mut().clear();
    }
}

/// Stdin backed by preloaded lines that echoes prompts and supplied lines to the screen.
struct ScreenStdin {
    screen: Rc<RefCell<TextScreen>>,
    lines: Rc<RefCell<VecDeque<String>>>,
    split_rule: LineSplitRule,
}

impl ScreenStdin {
    /// Prints `prompt` and removes the next preloaded line. Exhaustion returns `UnexpectedEof`, which the program's
    /// input intrinsic converts to an empty line.
    fn next_line(&self, prompt: &str) -> io::Result<String> {
        self.screen.borrow_mut().print(
            &plain_text(prompt, self.split_rule),
            PrintOptions::OPEN_LINE,
        );

        self.lines.borrow_mut().pop_front().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "The program reads more input than it was given.",
            )
        })
    }
}

impl Stdin for ScreenStdin {
    fn read_line(&self, prompt: &str, echo: Echo) -> InputFuture<String> {
        let line = self.next_line(prompt).inspect(|line| {
            let echo = match echo {
                Echo::Visible => line.clone(),
                Echo::Masked => "*".repeat(line.chars().count()),
                Echo::Hidden => String::new(),
            };

            self.screen.borrow_mut().print(
                &echo,
                PrintOptions {
                    new_line: true,
                    replace: false,
                },
            );
        });

        Box::pin(std::future::ready(line))
    }

    fn read_key(&self, prompt: &str) -> InputFuture<Key> {
        let key = self
            .next_line(prompt)
            .map(|line| line.chars().next().map_or(Key::Enter, Key::Char));

        Box::pin(std::future::ready(key))
    }
}
