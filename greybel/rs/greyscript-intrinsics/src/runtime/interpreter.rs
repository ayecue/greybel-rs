//! Runs Grey Hack programs against a server-owned world.

use std::cell::RefCell;
use std::pin::pin;
use std::rc::Rc;

use futures_util::future::{Either, select};
use greybel_core::Dialect;
use greybel_documents::AsyncResourceProvider;
use greybel_interpreter::{
    Clock, Interpreter, InterpreterError, InterpreterOptions, RunResult, Stdout, Vm,
};
use greybel_source_map::SourceMap;
use greyhack_behaviour::path;

use super::child::ChildRunner;
use super::{ServerConnection, register_game};
use crate::Game;
use crate::game::{CachedCode, LaunchCache, TerminalOutput, failure_text, launch_key};
use crate::protocol::{
    ClientEnd, EndProgram, Failure, Outcome, ProgramId, ScriptFailure, ShellWriteBinary,
    StartProgram,
};

/// Errors produced while driving the game interpreter.
#[derive(Debug, thiserror::Error)]
pub enum GreyHackError {
    /// Program execution failed.
    #[error(transparent)]
    Interpreter(#[from] InterpreterError),
    /// The game server connection closed or failed to exchange a message.
    #[error("the game server is unreachable: {0}")]
    GameServer(#[from] duplex_core::Error<ScriptFailure>),
    /// The program has not started and therefore has no file to write.
    #[error("the program has not started")]
    NoProgram,
    /// The program has no game state installed.
    #[error("the game world is not set up")]
    NoGame,
    /// The game refused to write the program file; contains the game's error text.
    #[error("{}", write_refusal_text(.0))]
    WriteRefusal(Failure),
}

impl From<duplex_core::Error> for GreyHackError {
    fn from(error: duplex_core::Error) -> Self {
        Self::GameServer(error.with_failure())
    }
}

/// Formats `failure` as Grey Hack's program-file write error.
fn write_refusal_text(failure: &Failure) -> String {
    failure_text("", failure).unwrap_or_else(|error| error.to_string())
}

/// Built program binary and source map for diagnostic line lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramBinary {
    pub code: String,
    pub source_map: SourceMap,
}

/// Grey Hack program connected to a game server.
pub struct GreyHackInterpreter<P> {
    interpreter: Interpreter<P>,
    server: ServerConnection,
    program: ProgramId,
    /// Program IDs for active launched contexts above the root.
    launches: Rc<RefCell<Vec<ProgramId>>>,
    /// Clock used for launch-tree throttle pauses.
    clock: Rc<dyn Clock>,
    /// Launch-tree terminal output.
    output: Rc<TerminalOutput>,
}

impl<P: AsyncResourceProvider> GreyHackInterpreter<P> {
    /// Starts a server-side process as `user` and builds its interpreter from `provider`.
    /// The program has no world file; Grey Hack identifies it as `script` in the user's home folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the game server does not respond.
    pub async fn start(
        server: &ServerConnection,
        user: &str,
        options: InterpreterOptions,
        provider: P,
    ) -> Result<Self, GreyHackError> {
        Self::start_at(server, user, None, options, provider).await
    }

    /// Equivalent to [`Self::start`], but identifies the program by `path` and writes its file with [`Self::install_binary`].
    ///
    /// # Errors
    ///
    /// Returns an error if the game server does not respond.
    pub async fn start_at(
        server: &ServerConnection,
        user: &str,
        path: Option<&str>,
        options: InterpreterOptions,
        provider: P,
    ) -> Result<Self, GreyHackError> {
        Self::start_in_terminal(server, user, path, false, options, provider).await
    }

    /// Equivalent to [`Self::start_at`], with the program typed into the safe-mode terminal when `safe_terminal`
    /// is set and the player's computer runs one (`GreyInterpreter.isSafeTerminal`). Programs it launches run in
    /// the same terminal.
    ///
    /// # Errors
    ///
    /// Returns an error if the game server does not respond.
    pub async fn start_in_terminal(
        server: &ServerConnection,
        user: &str,
        path: Option<&str>,
        safe_terminal: bool,
        options: InterpreterOptions,
        provider: P,
    ) -> Result<Self, GreyHackError> {
        let session = server
            .channel()
            .send_start_program(&StartProgram {
                user: user.into(),
                path: path.map(Into::into),
                safe_terminal,
            })?
            .await?;
        let options = InterpreterOptions {
            dialect: Dialect::GreyScript,
            ..options
        };
        let program = session.program;
        let clock = Rc::clone(&options.clock);

        let output = Rc::new(TerminalOutput::new(
            Rc::clone(&options.stdout),
            Rc::clone(&clock),
        ));
        let options = InterpreterOptions {
            stdout: Rc::clone(&output) as Rc<dyn Stdout>,
            ..options
        };

        server.register_output(program, Rc::clone(&output) as Rc<dyn Stdout>);

        let game = Game::new(server.clone(), session, Rc::clone(&output));
        let launches = Rc::clone(&game.launches);
        let runner = ChildRunner::new(&options);
        let interpreter = register_game(Interpreter::new(options, provider), runner, game);

        Ok(Self {
            interpreter,
            server: server.clone(),
            program,
            launches,
            clock,
            output,
        })
    }

    /// Controls `get_greybel_util()` availability.
    /// Enabled by default; disable for a normal game without the sandbox utility.
    #[must_use]
    pub fn with_admin(mut self, admin: bool) -> Self {
        self.interpreter.add_api(move |vm: &mut Vm| {
            if let Some(game) = vm.host_data_mut::<Game>() {
                game.admin = admin;
            }
        });

        self
    }

    /// Writes `binary` to the started program's path like Grey Hack's `build`.
    /// Launching that file or a copy runs this code; diagnostics and breakpoints use `binary`'s source map to name the original program files.
    ///
    /// # Errors
    ///
    /// Returns the game's file-write error or a game-server communication error.
    pub async fn install_binary(&mut self, binary: &ProgramBinary) -> Result<(), GreyHackError> {
        let vm = self.interpreter.vm_mut().ok_or(GreyHackError::NoProgram)?;
        let game = vm.host_data::<Game>().ok_or(GreyHackError::NoGame)?;
        let program_path = game.session.program_path.clone();
        let write = ShellWriteBinary {
            program: self.program,
            shell: game.session.shell.reference(),
            folder: path::parent(&program_path).to_owned().into(),
            name: path::file_name(&program_path).to_owned().into(),
            source: binary.code.clone().into(),
            allow_import: false,
            source_map: (&binary.source_map).into(),
        };
        let response = self
            .server
            .channel()
            .send_shell_write_binary(&write)?
            .await?;

        if let Outcome::Failure(failure) = response {
            return Err(GreyHackError::WriteRefusal(failure));
        }

        let entry = vm.program().entry();
        let launches = Rc::clone(&vm.host_data_or_insert_with(LaunchCache::default).0);

        launches.borrow_mut().insert(
            launch_key(&program_path, &binary.code),
            CachedCode {
                entry,
                source_map: SourceMap::default(),
            },
        );

        Ok(())
    }

    /// Returns the server-side process ID of the root program.
    #[must_use]
    pub fn program(&self) -> &ProgramId {
        &self.program
    }

    /// Returns the underlying base interpreter.
    #[must_use]
    pub fn interpreter(&self) -> &Interpreter<P> {
        &self.interpreter
    }

    pub fn interpreter_mut(&mut self) -> &mut Interpreter<P> {
        &mut self.interpreter
    }

    /// Flushes program output to the terminal while stopped, matching Grey Hack's debugger-evaluation run loop.
    pub fn send_output(&self) {
        self.output.send();
    }

    /// Reports whether the game server ended the root program; later interpreter errors belong to that shutdown.
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.server.has_ended(self.program)
    }

    /// Reports `reason` for the root process and waits for the server to commit the world.
    pub async fn end(&self, reason: ClientEnd) {
        self.server.forget(self.program);

        if let Ok(end) = self.server.channel().send_end_program(&EndProgram {
            program: self.program,
            reason,
        }) {
            let _ = end.await;
        }
    }

    /// Resumes the root context and descendants until a breakpoint or exit.
    /// Server-ended contexts unwind through [`Vm::exit_contexts_n`]; an ended root completes the run (see [`GreyHackInterpreter::has_ended`]).
    /// Tree throttles pause on the clock at the next yield or pending call.
    ///
    /// # Errors
    ///
    /// Returns root-program runtime errors.
    pub async fn resume(&mut self) -> Result<RunResult, InterpreterError> {
        loop {
            let notice_count = self.server.notice_count();

            self.apply_program_ends();

            let notice = pin!(self.server.notice_after(notice_count));

            if let Some(pause) = self.server.take_pause(self.tree()) {
                let _ = select(pin!(self.clock.sleep(pause)), notice).await;

                continue;
            }

            let run = pin!(self.interpreter.resume());

            if let Either::Left((result, _)) = select(run, notice).await {
                // Grey Hack's run loop flushes output while stopped and after termination.
                self.output.send();

                return result;
            }
        }
    }

    /// Root program and its active launches.
    fn tree(&self) -> Vec<ProgramId> {
        std::iter::once(self.program)
            .chain(self.launches.borrow().iter().copied())
            .collect()
    }

    /// Terminates the shallowest server-ended context and its descendants through [`Vm::exit_contexts_n`].
    fn apply_program_ends(&mut self) {
        let depth = std::iter::once(self.program)
            .chain(self.launches.borrow().iter().copied())
            .position(|program| self.server.has_ended(program));

        if let (Some(depth), Some(vm)) = (depth, self.interpreter.vm_mut()) {
            vm.exit_contexts_n(depth);
        }
    }

    /// Starts and runs the root program, then reports process end after the server commits the world.
    /// A server-ended root is acknowledged and returns [`RunResult::Complete`].
    ///
    /// # Errors
    ///
    /// Returns root-program runtime errors.
    pub async fn run(&mut self) -> Result<RunResult, GreyHackError> {
        let outcome = match self.interpreter.start().await {
            Ok(()) => self.resume().await,
            Err(error) => Err(error),
        };

        if self.has_ended() {
            self.end(ClientEnd::Acknowledged).await;

            return Ok(RunResult::Complete);
        }

        self.end(match outcome {
            Ok(_) => ClientEnd::Finished,
            Err(_) => ClientEnd::Error,
        })
        .await;

        Ok(outcome?)
    }
}
