//! Executes programs in Grey Hack single-player through the message-hook plugin.

mod debugger;
mod files;
mod inspection;
mod keys;

use std::pin::pin;
use std::rc::Rc;

use files::GameFile;
use futures_util::future::{Either, select};
use greybel_documents::Uri;
use greybel_io::{Echo, Key, PrintOptions};
use greyhack_message_hook_client::{
    Client, ClientOptions, Context, ContextBreakpoint, ContextOptions, GreybelNotification, Payload,
};
use inspection::GameInspection;
use keys::GameKey as _;

use super::binary;
use crate::protocol::{Exit, Launch, LaunchFailure, StopReason};
use crate::runtime::{CommandQueue, Host, Io, RuntimeState};
use crate::uri::EditorUri as _;

/// State of one Grey Hack message-hook launch.
pub(crate) struct GameProgram<'r, H: Host> {
    state: &'r Rc<RuntimeState<H>>,
    commands: &'r mut CommandQueue,
    launch: &'r Launch<'static>,
    provider: H::Provider,
    io: Io,
    /// Game stop state retained until resume.
    inspection: Option<GameInspection>,
}

impl<'r, H: Host> GameProgram<'r, H> {
    /// Connects to the message hook on `port`, runs `launch` to exit, then shuts down the client.
    pub(crate) async fn run(
        state: &'r Rc<RuntimeState<H>>,
        commands: &'r mut CommandQueue,
        launch: &'r Launch<'static>,
        port: u16,
        program_name: &str,
    ) -> Result<Exit, LaunchFailure> {
        let client = Client::connect(ClientOptions::with_port(port))
            .await
            .map_err(|error| {
                LaunchFailure::unreachable(format!(
                    "Grey Hack's message hook doesn't respond on port {port}: {error}"
                ))
            })?;
        let mut program = Self {
            state,
            commands,
            launch,
            provider: state.provider(launch),
            io: state.host.io(),
            inspection: None,
        };
        let outcome = program.start(&client, program_name).await;

        client.shutdown().await;

        outcome
    }

    /// Checks single-player mode, creates and runs a game context to exit, then disposes it.
    async fn start(&mut self, client: &Client, program_name: &str) -> Result<Exit, LaunchFailure> {
        let status = client.health_check().await.map_err(|error| {
            LaunchFailure::unreachable(format!("Grey Hack's message hook doesn't respond: {error}"))
        })?;

        if !status.is_single_player {
            return Err(LaunchFailure::unreachable(
                "Grey Hack has no active single-player game; start one to run programs.",
            ));
        }

        let options = self.context_options(program_name).await?;
        let context = client.create_context(options).await.map_err(|error| {
            LaunchFailure::unreachable(format!("Grey Hack didn't start the program: {error}"))
        })?;

        if self.launch.debug && self.launch.stop_on_entry {
            self.state.stop_reason.set(Some(StopReason::Entry));

            let _ = context.set_debug_mode(true).await;
        }

        let exit = self.handle(&context).await;
        let _ = context.dispose().await;

        Ok(exit)
    }

    /// Builds context options containing the launch binary, game-path source map, parameters, paths, debugger settings,
    /// and breakpoints. The build applies the launch's variables.
    async fn context_options(&self, program_name: &str) -> Result<ContextOptions, LaunchFailure> {
        let launch = self.launch;
        let entry = Uri::parse(&launch.entry.canonical())
            .map_err(|error| LaunchFailure::invalid(error.to_string()))?;
        let binary = binary::name_sources(
            binary::build(launch, &self.provider, true).await?,
            GameFile::game_path,
        );
        let filepath = entry.game_path();
        let basepath = match &launch.root {
            Some(root) => Uri::parse(&root.canonical())
                .map(|root| root.game_path())
                .unwrap_or_default(),
            None => filepath
                .rsplit_once('/')
                .map_or_else(String::new, |(folder, _)| folder.to_owned()),
        };

        Ok(ContextOptions {
            source: binary.code,
            source_map: binary.source_map,
            strict_mode: launch.strict_mode,
            params: launch.params.iter().map(ToString::to_string).collect(),
            filepath,
            basepath,
            program_name: program_name.to_owned(),
            debug_mode: launch.debug,
            breakpoints: self.breakpoints(),
        })
    }

    /// Converts editor breakpoint URIs into game paths and lines.
    fn breakpoints(&self) -> Vec<ContextBreakpoint> {
        let breakpoints = self.state.breakpoints.borrow();

        breakpoints
            .all_lines()
            .filter_map(|(uri, line)| {
                Some(ContextBreakpoint {
                    filepath: Uri::parse(uri).ok()?.game_path(),
                    line_num: i32::try_from(line).unwrap_or(i32::MAX),
                })
            })
            .collect()
    }

    /// Processes game notifications and editor commands until program exit.
    async fn handle(&mut self, context: &Context) -> Exit {
        let mut events = context.events();

        loop {
            let next = {
                let event = pin!(events.next());
                let command = pin!(self.commands.recv());

                match select(event, command).await {
                    Either::Left((event, _)) => Either::Left(event),
                    Either::Right((command, _)) => Either::Right(command),
                }
            };
            let exit = match next {
                Either::Left(Some(event)) => self.event(context, event).await,
                Either::Left(None) => Some(Exit::Crash {
                    message: "Grey Hack's message hook closed the connection.".to_owned(),
                    stack: Vec::new(),
                }),
                Either::Right(command) => self.command(context, command).await,
            };

            if let Some(exit) = exit {
                return exit;
            }
        }
    }

    /// Handles a game notification; program exit or connection closure returns `Some(Exit)`.
    async fn event(&mut self, context: &Context, event: GreybelNotification) -> Option<Exit> {
        match event {
            GreybelNotification::PrintSentClientRpc(print) => {
                // A replacement batch overwrites all terminal content (`TerminalListAdapter.AddText`).
                if print.replace_text {
                    self.io.stdout.clear();
                }

                self.io
                    .stdout
                    .print(&print.zip_output.text(), PrintOptions::default());
            }
            GreybelNotification::ClearScreenClientRpc(_) => self.io.stdout.clear(),
            GreybelNotification::InputSentClientRpc(input) => {
                let prompt = input.zip_output.text();
                let read = async {
                    if input.any_key {
                        self.io.stdin.read_key(&prompt).await.map(Key::game_name)
                    } else {
                        let echo = if input.is_password {
                            Echo::Masked
                        } else {
                            Echo::Visible
                        };

                        self.io.stdin.read_line(&prompt, echo).await
                    }
                };
                let read = pin!(read);
                let termination = pin!(self.commands.wait_for_termination(|| {}));

                match select(read, termination).await {
                    Either::Left((line, _)) => {
                        let _ = context.send_input(&line.unwrap_or_default()).await;
                    }
                    Either::Right(_) => return Some(Exit::Termination),
                }
            }
            GreybelNotification::ContextBreakpointRpc(stop) => self.stop(context, &stop).await,
            GreybelNotification::FinishedContextRpc(outcome) => {
                return Some(if outcome.failed {
                    Exit::Crash {
                        message: "The program failed in Grey Hack.".to_owned(),
                        stack: Vec::new(),
                    }
                } else {
                    Exit::Completion
                });
            }
            GreybelNotification::ContextUnexpectedErrorRpc(error) => {
                return Some(Exit::Crash {
                    message: error.message.into_owned(),
                    stack: Vec::new(),
                });
            }
            GreybelNotification::DisposedContextRpc(_) => return Some(Exit::Termination),
            _ => {}
        }

        None
    }
}
