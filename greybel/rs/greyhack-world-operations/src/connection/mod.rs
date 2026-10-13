//! Runtime connection state, operation routing, and script-object storage.

mod lifecycle;
pub(crate) mod objects;
mod operation;

use std::rc::{Rc, Weak};

use duplex_core::{
    Channel, Driver, Exchange, Message, Responder, ValidatedDecode, ValidatedEncode, ValueResponse,
};
use greyhack_world::{ComputerId, World};
use objects::ObjectTable;
pub(crate) use operation::Operation;
use scheduler::{LocalTask, Scheduler};

use crate::protocol::{
    GameServer, ObjectId, ProgramId, ProgramThrottle, ReleaseObjects, ScriptFailure,
};
use crate::server::{ServerState, WorldState};
use crate::{OperationError, Result, classes};

/// Runtime connection with its programs' script objects.
pub(crate) struct Connection {
    state: Weak<ServerState>,
    pub(crate) id: u32,
    /// Task spawner bound to the connection transport's scheduler.
    spawn: Spawn,
}

/// Running program context: its shell object, terminal computers, and root process.
pub(crate) struct Caller {
    /// The program that sent the operation.
    pub(crate) program: u32,
    pub(crate) session: ObjectId,
    pub(crate) terminal_computers: Vec<ComputerId>,
    pub(crate) root_process: Option<(ComputerId, u32)>,
    /// The caller's process when it is a command a terminal runs (`HelperScript.subPID`).
    pub(crate) terminal_command_process: Option<(ComputerId, u32)>,
}

/// Task-spawning callback for a transport scheduler.
pub(crate) type Spawn = Rc<dyn Fn(LocalTask)>;

/// Creates a task spawner from `channel`'s transport scheduler.
pub(crate) fn spawner<D: Driver>(channel: &Channel<GameServer, D>) -> Spawn {
    let scheduler = channel.driver().scheduler().clone();

    Rc::new(move |task| scheduler.spawn(task))
}

/// Registers operation and object-release handlers for connection `id` on `channel`.
pub(crate) fn attach<D: Driver>(
    state: &Rc<ServerState>,
    id: u32,
    channel: &Channel<GameServer, D>,
) {
    let connection = Rc::new(Connection {
        state: Rc::downgrade(state),
        id,
        spawn: spawner(channel),
    });
    let releases = Rc::clone(&connection);

    channel.on::<ReleaseObjects>(move |message, _next| {
        let (connection, objects) = (Rc::clone(&releases), message.objects.to_vec());

        (releases.spawn)(Box::pin(async move {
            if let Some(state) = connection.state() {
                state.release(connection.id, &objects).await;
            }
        }));
    });
    lifecycle::attach(&connection, channel);
    classes::attach(&connection, channel);
}

/// Client peer used to send operation messages.
type Client = <GameServer as duplex_core::Peer>::Remote;

/// Operation result `R` or raised script failure.
type OperationResponse<R> = ValueResponse<R, ScriptFailure>;

/// Responder associated with message `M`.
type ResponderOf<M> = <<M as Message>::Exchange as Exchange>::Responder;

/// Handlers resolve object IDs in the connection table and register returned objects. Messages with
/// unknown objects are dropped. Database failures also drop the message after recording the error for
/// [`WorldServer::save`](crate::WorldServer::save).
impl Connection {
    /// Upgrades the server reference, returning `None` after the server is dropped.
    pub(crate) fn state(&self) -> Option<Rc<ServerState>> {
        self.state.upgrade()
    }

    /// Handles each message `M` in a separate task and returns `run`'s result or
    /// script failure. Used for operations that wait outside cycles, such as deferred jobs; `run` manages
    /// its own cycles.
    pub(crate) fn operate<D, M, R>(
        self: &Rc<Self>,
        channel: &Channel<GameServer, D>,
        run: impl AsyncFn(&Rc<ServerState>, &Self, M) -> Result<R> + 'static,
    ) where
        D: Driver,
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
    {
        let connection = Rc::clone(self);
        let run = Rc::new(run);
        let output = channel.clone();

        channel.handle(move |message: M, responder: ResponderOf<M>, _next| {
            let (connection, run, output) =
                (Rc::clone(&connection), Rc::clone(&run), output.clone());
            let spawn = Rc::clone(&connection.spawn);

            spawn(Box::pin(async move {
                let Some(state) = connection.state() else {
                    return;
                };
                let program = message.program();
                let result = run(&state, &connection, message).await;

                respond(&state, &output, (connection.id, program), responder, result);
            }));
        });
    }

    /// Runs `read` in a separate world cycle for each message `M`, using the running program's caller
    /// context, and returns its result.
    pub(crate) fn read<D, M, R>(
        self: &Rc<Self>,
        channel: &Channel<GameServer, D>,
        read: impl AsyncFn(&World, &mut ObjectTable, M, &Caller) -> Result<R> + 'static,
    ) where
        D: Driver,
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
    {
        let read = Rc::new(read);

        self.cycle(
            channel,
            async move |state, connection, message: M, caller| {
                let (world, objects) = state.of(connection);

                read(world, objects, message, caller).await
            },
        );
    }

    /// Runs `change` in a separate world cycle for each message `M`, using the running program's caller
    /// context, and returns its result.
    pub(crate) fn change<D, M, R>(
        self: &Rc<Self>,
        channel: &Channel<GameServer, D>,
        change: impl AsyncFn(&mut World, &mut ObjectTable, M, &Caller) -> Result<R> + 'static,
    ) where
        D: Driver,
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
    {
        let change = Rc::new(change);

        self.cycle(
            channel,
            async move |state, connection, message: M, caller| {
                let (world, objects) = state.of(connection);

                change(world, objects, message, caller).await
            },
        );
    }

    /// Runs `apply` for each message `M` in its own cycle. The source program must still be running, and
    /// the response is sent before the cycle ends.
    fn cycle<D, M, R>(
        self: &Rc<Self>,
        channel: &Channel<GameServer, D>,
        apply: impl AsyncFn(&mut WorldState, u32, M, &Caller) -> Result<R> + 'static,
    ) where
        D: Driver,
        M: Message<Sender = Client, Exchange = OperationResponse<R>>
            + Operation
            + ValidatedDecode
            + 'static,
        R: ValidatedEncode + ValidatedDecode + 'static,
    {
        let connection = Rc::clone(self);
        let apply = Rc::new(apply);
        let output = channel.clone();

        channel.handle(move |message: M, responder: ResponderOf<M>, _next| {
            let (connection, apply, output) =
                (Rc::clone(&connection), Rc::clone(&apply), output.clone());
            let spawn = Rc::clone(&connection.spawn);

            spawn(Box::pin(async move {
                let Some(state) = connection.state() else {
                    return;
                };
                let program = message.program();

                state
                    .respond(
                        connection.id,
                        program,
                        async |world_state| {
                            async {
                                let program = program
                                    .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;
                                let caller = Caller {
                                    program: program.id,
                                    session: state.session(connection.id, program)?,
                                    terminal_computers: state
                                        .terminal_computers(connection.id, program),
                                    root_process: state.root_process(connection.id, program),
                                    terminal_command_process: state
                                        .terminal_command_process(connection.id, program),
                                };

                                apply(world_state, connection.id, message, &caller).await
                            }
                            .await
                        },
                        |result| {
                            respond(&state, &output, (connection.id, program), responder, result);
                        },
                    )
                    .await;
            }));
        });
    }
}

/// Returns `result` or its script failure.
/// Database errors are recorded and drop the response. Programs whose trees waited for budget receive a
/// throttle afterward.
fn respond<D: Driver, R: ValidatedEncode>(
    state: &ServerState,
    channel: &Channel<GameServer, D>,
    (connection, program): (u32, Option<ProgramId>),
    responder: Responder<R, ScriptFailure>,
    result: Result<R>,
) {
    match result {
        Ok(value) => responder.respond(value),
        Err(OperationError::Script(failure)) => responder.fail(failure),
        Err(OperationError::World(error)) => state.record_failure(error),
        Err(OperationError::UnknownObject) => {}
    }

    if let Some(program) = program
        && let Some(milliseconds) = state.take_throttle(connection, program)
    {
        let _ = channel.send(&ProgramThrottle {
            program,
            milliseconds,
        });
    }
}
