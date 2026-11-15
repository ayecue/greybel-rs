//! Executes programs through the Grey Hack host in test worlds.

use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions};
use duplex_test_support::local::run_local;
use greybel_interpreter::{InterpreterError, InterpreterOptions, Stdout};
use greyhack_world::{DatabaseOpen, World};
use greyhack_world_core::Database;
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::protocol::Program;
use greyscript_intrinsics::{GreyHackError, GreyHackInterpreter, ServerConnection};

use crate::interpreter::{constant_clock, inline};
use crate::io::MarkedStdout;

/// Host-run output, runtime error, and final world state.
pub struct HostRun {
    /// Text captured by [`MarkedStdout`].
    pub output: String,
    /// Final program terminal contents ([`MarkedStdout::terminal_text`]).
    pub terminal: String,
    /// Runtime error message when execution fails.
    pub error: Option<String>,
    /// In-memory snapshot of the post-execution world.
    pub world: World,
}

/// Runs `code` as `user` with optional admin rights, an in-memory world database, and marked output. Runtime
/// errors are returned in `HostRun`; other failures panic.
pub fn run_on_host(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    options: InterpreterOptions,
) -> HostRun {
    run_on_host_within(code, world, user, admin, options, ())
}

/// Equivalent to [`run_on_host`], but identifies the program by `path` just as editor-run programs are identified
/// by their binary.
pub fn run_on_host_at(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    path: &str,
    options: InterpreterOptions,
) -> HostRun {
    run_on_server(code, world, user, admin, Some(path), options)
}

/// Async equivalent of [`run_on_host_at`] for a current-thread runtime or [`tokio::task::LocalSet`].
pub async fn run_on_host_at_async(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    path: &str,
    options: InterpreterOptions,
) -> HostRun {
    run_on_terminal_server_async(code, world, user, admin, (Some(path), false), options).await
}

/// Equivalent to [`run_on_host`]. Ignores the final parameter, formerly an entity-cache limit before that cache
/// was removed.
pub fn run_on_host_within(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    options: InterpreterOptions,
    _capacity: (),
) -> HostRun {
    run_on_server(code, world, user, admin, None, options)
}

/// Equivalent to [`run_on_host`], with the program typed into the safe-mode terminal the world runs
/// (`GreyInterpreter.isSafeTerminal`).
pub fn run_on_host_in_safe_terminal(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    options: InterpreterOptions,
) -> HostRun {
    run_on_terminal_server(code, world, user, admin, (None, true), options)
}

/// Runs `code` as program `path` on a new `world` server; `None` selects the default program path.
fn run_on_server(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    path: Option<&str>,
    options: InterpreterOptions,
) -> HostRun {
    run_on_terminal_server(code, world, user, admin, (path, false), options)
}

/// Runs `code` on a new `world` server as program `path` (`None` selects the default program path) typed into the
/// safe-mode terminal when the flag is set.
fn run_on_terminal_server(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    (path, safe_terminal): (Option<&str>, bool),
    options: InterpreterOptions,
) -> HostRun {
    run_local(run_on_terminal_server_async(
        code,
        world,
        user,
        admin,
        (path, safe_terminal),
        options,
    ))
}

/// Runs a host program on the caller's current-thread runtime.
async fn run_on_terminal_server_async(
    code: &str,
    world: World,
    user: &str,
    admin: bool,
    (path, safe_terminal): (Option<&str>, bool),
    options: InterpreterOptions,
) -> HostRun {
    let stdout = Rc::new(MarkedStdout::default());
    let options = InterpreterOptions {
        stdout: Rc::clone(&stdout) as Rc<dyn Stdout>,
        ..options
    };

    {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            constant_clock(),
        )
        .await
        .expect("the world is written to the database");

        let error = run_program(&server, code, user, admin, (path, safe_terminal), options).await;
        let world = World::load(server.database().expect("a database"))
            .await
            .expect("the world loads");

        HostRun {
            output: stdout.text(),
            terminal: stdout.terminal_text(),
            error,
            world,
        }
    }
}

/// Runs `code` on existing `server` with optional admin rights on the current `LocalSet`. Returns a runtime error
/// or `None` on success; other failures panic.
pub async fn run_in(
    server: &WorldServer,
    code: &str,
    user: &str,
    admin: bool,
    options: InterpreterOptions,
) -> Option<String> {
    run_program(server, code, user, admin, (None, false), options).await
}

async fn run_program(
    server: &WorldServer,
    code: &str,
    user: &str,
    admin: bool,
    (path, safe_terminal): (Option<&str>, bool),
    options: InterpreterOptions,
) -> Option<String> {
    let mut host = GreyHackInterpreter::start_in_terminal(
        &connect(server),
        user,
        path,
        safe_terminal,
        options,
        inline(code),
    )
    .await
    .expect("the program starts")
    .with_admin(admin);

    match host.run().await {
        Ok(_) => None,
        Err(GreyHackError::Interpreter(InterpreterError::Runtime(error))) => {
            Some(error.to_string())
        }
        Err(other) => panic!("{other}"),
    }
}

/// Calls [`run_in`] with program identity `path`.
pub async fn run_in_at(
    server: &WorldServer,
    code: &str,
    user: &str,
    admin: bool,
    path: &str,
    options: InterpreterOptions,
) -> Option<String> {
    run_program(server, code, user, admin, (Some(path), false), options).await
}

/// Calls [`run_in_at`] with the program typed into the safe-mode terminal the world runs
/// (`GreyInterpreter.isSafeTerminal`).
pub async fn run_in_at_safe_terminal(
    server: &WorldServer,
    code: &str,
    user: &str,
    admin: bool,
    path: &str,
    options: InterpreterOptions,
) -> Option<String> {
    run_program(server, code, user, admin, (Some(path), true), options).await
}

/// Creates a same-thread, in-memory connection from a program to `server`.
#[must_use]
pub fn connect(server: &WorldServer) -> ServerConnection {
    ServerConnection::new(Channel::<Program, _>::new(
        &server.connect_on_tokio(),
        ChannelOptions::default(),
    ))
}
