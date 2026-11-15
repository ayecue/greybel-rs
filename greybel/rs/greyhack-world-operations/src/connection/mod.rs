//! Runtime connection state, operation routing, and script-object storage.

mod lifecycle;

use std::rc::{Rc, Weak};

use duplex_core::{
    Channel, Dispatch, Driver, NextMessage, NextNotification, NotificationMessage, Responder,
    ValidatedDecode, ValidatedEncode,
};
use greyhack_world::World;
use greyhack_world_core::WorldError;
use greyhack_world_programs::protocol::{
    GameServer, ProgramId, ProgramThrottle, ReleaseObjects, ScriptFailure,
};
use greyhack_world_router::{
    Call, CycleAccess, Operation, OperationResponse, ResponderOf, RouteCall, RouteContext,
    RouteKind,
};
use scheduler::{LocalTask, Scheduler};

use crate::server::ServerState;
use crate::{OperationError, Result};

/// Runtime connection with its program trees.
pub(crate) struct Connection {
    state: Weak<ServerState>,
    pub(crate) id: u32,
    /// Task spawner bound to the connection transport's scheduler.
    spawn: Spawn,
    /// Sends throttling notices through the connection's channel.
    throttle: Rc<dyn Fn(ProgramThrottle)>,
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
    let throttle = {
        let channel = channel.clone();

        move |notice| {
            let _ = channel.send(&notice);
        }
    };
    let connection = Rc::new(Connection {
        state: Rc::downgrade(state),
        id,
        spawn: spawner(channel),
        throttle: Rc::new(throttle),
    });
    let releases = Rc::clone(&connection);

    channel.on::<ReleaseObjects>(move |message, _next| {
        let (connection, objects) = (Rc::clone(&releases), message.objects.to_vec());

        (releases.spawn)(Box::pin(async move {
            if let Some(state) = connection.state() {
                state.release(connection.id, &objects);
            }
        }));
    });
    lifecycle::attach(&connection, channel);
    greyhack_world_programs::protocol::attach_dispatch(channel, connection.clone());
}

/// Handlers resolve object IDs in their program tree and register returned objects. Messages with unknown
/// objects are dropped. Database failures also drop the message after recording the error for
/// [`WorldServer::save`](crate::WorldServer::save).
impl Connection {
    /// Upgrades the server reference, returning `None` after the server is dropped.
    pub(crate) fn state(&self) -> Option<Rc<ServerState>> {
        self.state.upgrade()
    }

    /// Runs a lifecycle message in its own task and returns its typed response.
    pub(crate) fn operate<D, M, R>(
        self: &Rc<Self>,
        channel: &Channel<GameServer, D>,
        run: impl AsyncFn(&Rc<ServerState>, &Self, M) -> Result<R> + 'static,
    ) where
        D: Driver,
        M: duplex_core::Message<
                Sender = <GameServer as duplex_core::Peer>::Remote,
                Exchange = OperationResponse<R>,
            > + Operation
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

                if let Some(program) = program
                    && matches!(
                        result,
                        Err(OperationError::Script(ScriptFailure::ConnectionInterrupted))
                    )
                {
                    state
                        .cycle(
                            async |world_state| {
                                state
                                    .close_lost_terminal(
                                        world_state,
                                        connection.id,
                                        program,
                                        &result,
                                        false,
                                    )
                                    .await;
                            },
                            true,
                        )
                        .await;
                }

                respond_lifecycle(&state, &output, (connection.id, program), responder, result);
            }));
        });
    }

    /// Runs a one-cycle typed route package and responds after its effects settle.
    fn cycle_route(self: &Rc<Self>, route: Box<dyn RouteCall<World, WorldError>>, commit: bool) {
        let connection = Rc::clone(self);
        let spawn = Rc::clone(&connection.spawn);

        spawn(Box::pin(async move {
            let Some(state) = connection.state() else {
                return;
            };
            let program = route.program();
            let key = program.and_then(|program| state.trees.of_program(program));

            state
                .respond(
                    connection.id,
                    program,
                    async |world_state| {
                        async {
                            let program = program
                                .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;
                            let key =
                                key.ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;

                            if world_state
                                .tree(key)
                                .is_none_or(|tree| tree.active(program).is_none())
                            {
                                return Err(OperationError::Script(ScriptFailure::ProgramEnded));
                            }

                            world_state
                                .tree_mut(key)
                                .expect("the indexed tree exists")
                                .begin_route();

                            let call = Call { tree: key, program };
                            let context = RouteContext::new(
                                world_state.world.clone(),
                                world_state.trees.clone(),
                            )
                            .with_prints(state.prints.clone());
                            let result = if route.kind() == RouteKind::Read {
                                world_state
                                    .world
                                    .handle
                                    .read_route(async || route.run(&context, call).await)
                                    .await
                            } else {
                                route.run(&context, call).await
                            };
                            let terminal_reboot = world_state
                                .tree_mut(key)
                                .expect("the indexed tree exists")
                                .take_effects()
                                .terminal_rebooted;

                            state
                                .close_lost_terminal(
                                    world_state,
                                    connection.id,
                                    program,
                                    &result,
                                    terminal_reboot,
                                )
                                .await;

                            result
                        }
                        .await
                    },
                    |result| respond(&state, &connection, program, &*route, result),
                    commit,
                )
                .await;
        }));
    }

    /// Runs a wait route in tasks and cycles managed by its handlers.
    fn operate_route(self: &Rc<Self>, route: Box<dyn RouteCall<World, WorldError>>) {
        let connection = Rc::clone(self);
        let spawn = Rc::clone(&connection.spawn);

        spawn(Box::pin(async move {
            let Some(state) = connection.state() else {
                return;
            };
            let program = route.program();
            let result = async {
                let program = program.ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;
                let tree = state
                    .trees
                    .of_program(program)
                    .ok_or(OperationError::Script(ScriptFailure::ProgramEnded))?;
                let (modules, trees) = {
                    let world = state.world.lock().await;

                    (world.world.clone(), world.trees.clone())
                };
                let context = RouteContext::with_cycles(
                    modules,
                    trees,
                    CycleAccess::new(connection.id, state.as_ref()),
                )
                .with_prints(state.prints.clone());

                route.run(&context, Call { tree, program }).await
            }
            .await;

            if let Some(program) = program
                && matches!(
                    result,
                    Err(OperationError::Script(ScriptFailure::ConnectionInterrupted))
                )
            {
                state
                    .cycle(
                        async |world_state| {
                            state
                                .close_lost_terminal(
                                    world_state,
                                    connection.id,
                                    program,
                                    &result,
                                    false,
                                )
                                .await;
                        },
                        true,
                    )
                    .await;
            }

            state.send_program_prints();
            respond(&state, &connection, program, &*route, result);
        }));
    }
}

impl Dispatch for Connection {
    fn message<M: duplex_core::Message + ValidatedDecode + 'static>(
        self: Rc<Self>,
        message: M,
        responder: ResponderOf<M>,
        next: NextMessage<M>,
    ) {
        let Some(state) = self.state() else {
            return;
        };

        if !state.routes.contains::<M>() {
            next.call(message, responder);

            return;
        }

        let route = state
            .routes
            .call::<M, World, WorldError>(message, responder)
            .expect("the route table contains the message");
        let kind = route.kind();

        match kind {
            RouteKind::Read => self.cycle_route(route, false),
            RouteKind::Change => self.cycle_route(route, true),
            RouteKind::Wait => self.operate_route(route),
        }
    }

    fn notification<N: NotificationMessage + 'static>(
        &self,
        message: N::Ref<'_>,
        next: NextNotification<N>,
    ) {
        next.call(message);
    }
}

/// Responds to a lifecycle message and applies the tree's throttle.
fn respond_lifecycle<D: Driver, R: ValidatedEncode>(
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

/// Returns `result` or its script failure. Database errors are recorded and drop the response. Programs whose
/// trees waited for budget receive a throttle afterward.
fn respond(
    state: &ServerState,
    connection: &Connection,
    program: Option<ProgramId>,
    route: &dyn RouteCall<World, WorldError>,
    result: Result<Box<dyn std::any::Any>>,
) {
    match result {
        Ok(value) => route.respond(Ok(value)),
        Err(OperationError::Script(failure)) => route.respond(Err(failure)),
        Err(OperationError::World(error)) => state.record_failure(error),
        Err(OperationError::UnknownObject) => {}
    }

    if let Some(program) = program
        && let Some(milliseconds) = state.take_throttle(connection.id, program)
    {
        (connection.throttle)(ProgramThrottle {
            program,
            milliseconds,
        });
    }
}
