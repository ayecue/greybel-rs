//! Executes runtime programs in a simulated Grey Hack world.

mod debugger;
mod entry;
mod inspection;

use std::cell::RefCell;
use std::pin::pin;
use std::rc::Rc;

use futures_util::future::{Either, select};
use greybel_core::SyntaxFeatures;
use greybel_documents::{AsyncResourceProvider, Uri};
use greybel_interpreter::{InterpreterError, InterpreterOptions, Param, Pin, RunResult, Value, Vm};
use greyhack_behaviour::path;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::protocol::ClientEnd;
use greyscript_intrinsics::{GreyHackInterpreter, ServerConnection};

use super::binary;
use crate::protocol::{Exit, Launch, LaunchFailure, SourceLocation, StopReason};
use crate::runtime::{CommandQueue, Host, Io, RuntimeState};
use crate::uri::EditorUri as _;

/// Native API global for expression values, named to avoid program-variable collisions.
const EVALUATION_GLOBAL: &str = "__greybel_evaluation";

/// Grey Hack interpreter using the host's document provider.
type MockGame<H> = GreyHackInterpreter<<H as Host>::Provider>;

/// Allows mutable VM access after startup.
trait StartedGame {
    fn vm(&mut self) -> &mut Vm;
}

impl<P: AsyncResourceProvider> StartedGame for GreyHackInterpreter<P> {
    fn vm(&mut self) -> &mut Vm {
        self.interpreter_mut().vm_mut().expect("a started program")
    }
}

/// State of one mock-world launch.
pub(crate) struct MockProgram<'r, H: Host> {
    state: &'r Rc<RuntimeState<H>>,
    commands: &'r mut CommandQueue,
    /// Latest pinned expression result captured through [`EVALUATION_GLOBAL`].
    evaluation_value: Rc<RefCell<Option<Pin>>>,
    /// Memory allowance in bytes before program interruption.
    memory_limit: usize,
}

impl<'r, H: Host> MockProgram<'r, H> {
    /// Runs `launch` in the host world or a world seeded by `seed`, handles debugger stops until exit, and limits
    /// memory to `memory_limit` MiB.
    pub(crate) async fn run(
        state: &'r Rc<RuntimeState<H>>,
        commands: &'r mut CommandQueue,
        launch: &Launch<'static>,
        seed: &str,
        memory_limit: u32,
    ) -> Result<Exit, LaunchFailure> {
        let mut program = Self {
            state,
            commands,
            evaluation_value: Rc::default(),
            memory_limit: usize::try_from(memory_limit)
                .map_or(usize::MAX, |limit| limit.saturating_mul(1 << 20)),
        };
        let (world, _server) = match state.host.world() {
            Some(world) => (world, None),
            None => {
                let (world, server) = own_world(&state.host, seed).await;

                (world, Some(server))
            }
        };
        let mut game = program.start(launch, &world).await?;
        let control = &state.control;

        if launch.debug {
            game.vm().set_debug_control(Rc::clone(control));
            if launch.stop_on_entry {
                state.stop_reason.set(Some(StopReason::Entry));
                control.pause();
            }
        }

        let (exit, end) = program.drive(&mut game, launch.debug).await;

        game.end(end).await;

        state.stop_reason.set(None);
        control.resume();

        Ok(exit)
    }

    /// Compiles and starts `launch` in `world`, installing [`EVALUATION_GLOBAL`] for expression-result capture.
    async fn start(
        &self,
        launch: &Launch<'static>,
        world: &ServerConnection,
    ) -> Result<MockGame<H>, LaunchFailure> {
        let Io { stdout, stdin } = self.state.host.io();
        let options = InterpreterOptions {
            target: launch.entry.canonical(),
            params: launch.params.iter().map(ToString::to_string).collect(),
            environment: launch
                .variables
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
            debug: launch.debug,
            strict_mode: launch.strict_mode,
            syntax_features: SyntaxFeatures::except(
                launch
                    .disabled_syntax
                    .iter()
                    .flat_map(|names| names.iter())
                    .map(String::as_str),
            ),
            memory_limit: self.memory_limit,
            stdout,
            stdin,
            ..InterpreterOptions::new(self.state.host.clock())
        };
        let player = PlayerOptions::default();
        let program_path = path::join(&path::home(&player.user), &launch.program_name);
        let mut game = GreyHackInterpreter::start_at(
            world,
            &player.user,
            Some(&program_path),
            options,
            self.state.provider(launch),
        )
        .await
        .map_err(|error| LaunchFailure::invalid(error.to_string()))?;
        let interpreter = game.interpreter_mut();
        let start_result = match interpreter.compile().await {
            Ok(_) => interpreter.start().await,
            Err(error) => Err(error),
        };

        if let Err(error) = start_result {
            let location = match &error {
                InterpreterError::Documents(error) => {
                    SourceLocation::of_syntax_error(error, &self.state.provider(launch)).await
                }
                _ => None,
            };

            return Err(LaunchFailure::Invalid {
                message: error.to_string(),
                location,
            });
        }

        let provider = self.state.provider(launch);
        // Format error filenames like the program's own errors.
        let binary =
            binary::name_sources(binary::build(launch, &provider, false).await?, Uri::label);

        game.install_binary(&binary)
            .await
            .map_err(|error| LaunchFailure::invalid(error.to_string()))?;

        let evaluation_value = Rc::clone(&self.evaluation_value);
        let vm = game.vm();
        let capture = vm.add_native([Param::new("value")], move |vm, arguments| {
            let pin = vm.pin(arguments.get("value").clone());

            if let Some(previous) = evaluation_value.borrow_mut().replace(pin) {
                vm.unpin(&previous);
            }

            Ok(Value::Null)
        });
        let api = vm.api();

        api.set(vm.heap_mut(), Value::from(EVALUATION_GLOBAL), capture);

        Ok(game)
    }

    /// Runs the interpreter alongside editor commands and returns protocol exit and server-end reasons. Pause stops
    /// execution only when `debug` is true.
    async fn drive(&mut self, game: &mut MockGame<H>, debug: bool) -> (Exit, ClientEnd) {
        let state = self.state;
        let pause = || {
            if debug {
                state.stop_reason.set(Some(StopReason::Pause));
                state.control.pause();
            }
        };

        loop {
            let result = {
                let resume = pin!(self.state.run_with_yields(game.resume()));
                let editor = pin!(self.commands.wait_for_termination(pause));

                match select(editor, resume).await {
                    Either::Left(_) => return (Exit::Termination, ClientEnd::Stopped),
                    Either::Right((result, _)) => result,
                }
            };

            match result {
                // The server stopped the program at its current instruction.
                Ok(RunResult::Complete) if game.has_ended() => {
                    return (Exit::Termination, ClientEnd::Acknowledged);
                }
                Err(_) if game.has_ended() => return (Exit::Completion, ClientEnd::Acknowledged),
                Ok(RunResult::Complete) => return (Exit::Completion, ClientEnd::Finished),
                Ok(RunResult::Breakpoint(location)) => {
                    if let Some(exit) = self.serve_stop(game, location).await {
                        return (exit, ClientEnd::Stopped);
                    }
                }
                Err(InterpreterError::Runtime(error)) => {
                    let exit = Exit::Crash {
                        message: error.to_string(),
                        stack: error.stack.iter().map(SourceLocation::from).collect(),
                    };

                    return (exit, ClientEnd::Error);
                }
                Err(error) => {
                    let exit = Exit::Crash {
                        message: error.to_string(),
                        stack: Vec::new(),
                    };

                    return (exit, ClientEnd::Error);
                }
            }
        }
    }
}

/// Creates an in-memory `WorldServer` from a seed and connects through `host`.
async fn own_world<H: Host>(host: &H, seed: &str) -> (ServerConnection, WorldServer) {
    let server = mock_world_server(seed, host.clock()).await;

    (host.connect(&server), server)
}

/// Creates an in-memory [`WorldServer`] from `seed`, generating the default player world.
pub async fn mock_world_server(seed: &str, clock: Rc<dyn greybel_interpreter::Clock>) -> WorldServer {
    let world = World::from_seed(seed, &PlayerOptions::default())
        .await
        .expect("the world generates");

    WorldServer::new(world, clock)
        .await
        .expect("the server restores payments")
}
