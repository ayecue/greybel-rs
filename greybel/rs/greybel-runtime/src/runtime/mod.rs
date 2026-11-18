//! Runtime state, controller commands, and service loop for one active program.

mod command;
mod controller;
mod host;
mod state;
mod yielder;

use std::pin::pin;
use std::rc::Rc;

pub(crate) use command::{Command, CommandQueue, Reply};
pub use controller::Controller;
use futures_util::future::{Either, select};
pub use host::{Host, Io};
pub(crate) use state::RuntimeState;
use tokio::sync::mpsc;
pub use yielder::Yielder;

#[cfg(feature = "game")]
use crate::program::GameProgram;
use crate::program::MockProgram;
use crate::protocol::{Environment, Exit, Launch, LaunchFailure, RunUser};

/// Executes one program at a time with host resources and controller commands.
pub struct Runtime<H: Host> {
    state: Rc<RuntimeState<H>>,
    commands: CommandQueue,
}

impl<H: Host> Runtime<H> {
    /// Creates a runtime/controller pair sharing state and a command queue for `host`.
    #[must_use]
    pub fn with_host(host: H) -> (Self, Controller<H>) {
        let (sender, commands) = mpsc::unbounded_channel();
        let state = Rc::new(RuntimeState::new(host));
        let controller = Controller::new(Rc::clone(&state), sender);

        (
            Self {
                state,
                commands: CommandQueue::new(commands),
            },
            controller,
        )
    }

    /// Sets the host event `yielder` invoked on program self-yields so messages such as pause can be processed between
    /// program tasks.
    #[must_use]
    pub fn with_yielder(self, yielder: Yielder) -> Self {
        self.state.yielder.replace(Some(yielder));

        self
    }

    /// Returns this runtime's host.
    #[must_use]
    pub fn host(&self) -> &H {
        &self.state.host
    }

    /// Processes launches until all controllers drop or the host close event completes.
    pub async fn serve(mut self) {
        let mut close_event = pin!(self.state.host.close_event());

        loop {
            let command = {
                let next = pin!(self.commands.recv());

                match select(next, close_event.as_mut()).await {
                    Either::Left((Some(command), _)) => command,
                    Either::Left((None, _)) | Either::Right(_) => return,
                }
            };

            match command {
                Command::Launch(launch, reply) => reply(self.launch(launch).await),
                command => command.refuse(),
            }
        }
    }

    /// Executes the launch in its selected environment until exit.
    async fn launch(&mut self, launch: Launch<'static>) -> Result<Exit, LaunchFailure> {
        self.state.breakpoints.borrow_mut().reset_hits();

        match &launch.environment {
            Environment::Mock {
                seed,
                memory_limit,
                user,
            } => {
                MockProgram::run(
                    &self.state,
                    &mut self.commands,
                    &launch,
                    seed,
                    *memory_limit,
                    user.unwrap_or(RunUser::User),
                )
                .await
            }
            #[cfg(feature = "game")]
            Environment::Game { port } => {
                GameProgram::run(
                    &self.state,
                    &mut self.commands,
                    &launch,
                    *port,
                    &launch.program_name,
                )
                .await
            }
            #[cfg(not(feature = "game"))]
            Environment::Game { .. } => Err(LaunchFailure::Unavailable),
        }
    }
}
