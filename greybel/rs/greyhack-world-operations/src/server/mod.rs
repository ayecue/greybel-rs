//! Game server for one shared world. Operations, ticks, saves, and connection cleanup run as cycles.
//! Cycles acquire the world in request order, apply their changes, and process resulting events in fixed
//! phases before releasing the world.

mod budget;
mod cycle;
mod cycles;
mod deferred_jobs;
mod lifecycle;
mod local;
mod ticks;
mod trees;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use budget::BudgetTable;
use deferred_jobs::DeferredJob;
use duplex_core::{Channel, ChannelOptions, Driver, Transport};
use greyhack_world::World;
use greyhack_world_core::{ComputerId, Database, RebootEvent, WorldError};
use greyhack_world_effects::{WorkCompletion, WorkEnvelope};
use greyhack_world_programs::protocol::{GameServer, GameServerChannel, ProgramEnd};
use greyhack_world_programs::{ProgramTree, ProgramTreePool, TreeId};
use greyhack_world_router::PrintQueue;
use scheduler::Clock;
pub use ticks::TickOptions;
use ticks::TickState;
use tokio::sync::{Mutex, MutexGuard};
use trees::TreeIndex;

use crate::connection::{self};

/// Shared-world server accepting runtime channels that execute operations on the same world.
#[derive(Clone)]
pub struct WorldServer(Rc<ServerState>);

/// Read-only world access while the server is locked.
pub struct WorldView<'a>(MutexGuard<'a, ServerWorld>);

impl std::ops::Deref for WorldView<'_> {
    type Target = World;

    fn deref(&self) -> &World {
        self.0.world.as_ref()
    }
}

pub(crate) struct ServerState {
    /// World and connection object tables locked for an entire cycle, including awaits. Lock waiters are
    /// served in request order.
    pub(crate) world: Mutex<ServerWorld>,
    pub(crate) index: RefCell<TreeIndex>,
    /// Program trees shared with route contexts and indexed before a cycle is locked.
    pub(crate) trees: Rc<ProgramTreePool<ComputerId>>,
    /// Domain routes installed on every connection.
    pub(crate) routes: Rc<greyhack_world_router::RouteTable>,
    /// Optional persistent world database.
    database: Option<Database>,
    /// Deferred database error returned by the next [`WorldServer::save`].
    failure: RefCell<Option<WorldError>>,
    /// Host clock used to schedule ticks.
    clock: Rc<dyn Clock>,
    ticks: TickState,
    /// Program-tree budgets and active world claims.
    budgets: BudgetTable,
    /// Typed output lines queued by routes, sent before their cycle's responses.
    pub(crate) prints: PrintQueue,
    /// Program termination notices queued for the end of the cycle, after operation responses.
    program_ends: RefCell<Vec<(ProgramEnd, ConnectionChannel)>>,
    /// Deferred operations keyed by scheduled work ID.
    deferred_jobs: RefCell<HashMap<greyhack_world_effects::WorkId, DeferredJob>>,
    /// Work results stored until a successful commit resolves their deferred jobs.
    pending_ends: RefCell<Vec<WorkEnvelope>>,
    /// RebootEvent computers whose programs end after object interruption.
    pending_reboots: RefCell<Vec<ComputerId>>,
}

/// World, program trees, and connection object tables the current cycle exclusively owns.
pub(crate) struct ServerWorld {
    pub(crate) world: Rc<World>,
    pub(crate) trees: Rc<ProgramTreePool<ComputerId>>,
}

impl ServerWorld {
    /// Returns the tree identified by `key`.
    pub(crate) fn tree(&self, key: TreeId) -> Option<std::cell::Ref<'_, ProgramTree<ComputerId>>> {
        self.trees.get(key)
    }

    /// Returns the mutable tree identified by `key`.
    pub(crate) fn tree_mut(
        &self,
        key: TreeId,
    ) -> Option<std::cell::RefMut<'_, ProgramTree<ComputerId>>> {
        self.trees.get_mut(key)
    }
}

impl WorldServer {
    /// Creates a server for `world` using `clock`. Generated worlds use their in-memory database, so
    /// [`WorldServer::save`] reports that nothing was saved.
    ///
    /// # Errors
    ///
    /// Returns an error if subscription payment timers cannot be restored.
    pub async fn new(world: World, clock: Rc<dyn Clock>) -> greyhack_world_core::Result<Self> {
        Self::on(world, clock, None).await
    }

    async fn on(
        world: World,
        clock: Rc<dyn Clock>,
        database: Option<Database>,
    ) -> greyhack_world_core::Result<Self> {
        world.modules.economy.bank.restore_payments().await?;

        let trees = Rc::new(ProgramTreePool::default());

        let events = world.assembly.events.clone();
        let routes = world.assembly.routes.clone();
        let state = Rc::new(ServerState {
            trees: trees.clone(),
            routes,
            database,
            world: Mutex::new(ServerWorld {
                world: Rc::new(world),
                trees,
            }),
            index: RefCell::new(TreeIndex::default()),
            failure: RefCell::new(None),
            clock,
            ticks: TickState::default(),
            budgets: BudgetTable::default(),
            prints: PrintQueue::default(),
            program_ends: RefCell::new(Vec::new()),
            deferred_jobs: RefCell::new(HashMap::new()),
            pending_ends: RefCell::new(Vec::new()),
            pending_reboots: RefCell::new(Vec::new()),
        });

        events.on({
            let state = Rc::downgrade(&state);

            move |event: &RebootEvent| {
                if let Some(state) = state.upgrade() {
                    state.trees.interrupt(event.computer);
                    state.pending_reboots.borrow_mut().push(event.computer);
                }
            }
        });
        events.on({
            let state = Rc::downgrade(&state);

            move |event: &WorkCompletion| {
                if let Some(state) = state.upgrade()
                    && let Some(end) = event.take_envelope()
                {
                    state.pending_ends.borrow_mut().push(end);
                }
            }
        });
        events.on(|_: &greyhack_world_traces::StrikeEvent| {});
        events.on(|_: &greyhack_world_player::GameOverEvent| {});

        Ok(Self(state))
    }

    /// Updates tick rate and work limits starting with the next tick.
    pub fn set_tick_options(&self, options: TickOptions) {
        self.0.ticks.options.set(options);
    }

    /// Processes ticks currently due on the host clock, allowing manual ticking after a runtime attaches.
    pub async fn tick(&self) {
        self.0.tick().await;
    }

    /// Saves `world` into `database`, then opens a server on that copy using `clock`.
    ///
    /// # Errors
    ///
    /// Returns errors produced while copying the world.
    pub async fn with_database(
        world: World,
        database: Database,
        clock: Rc<dyn Clock>,
    ) -> greyhack_world_core::Result<Self> {
        world.save_into(&database).await?;

        Self::on(World::load(&database).await?, clock, Some(database)).await
    }

    /// Loads a world from `database` and persists later changes while using `clock`.
    ///
    /// # Errors
    ///
    /// Returns world-loading errors.
    pub async fn open(
        database: Database,
        clock: Rc<dyn Clock>,
    ) -> greyhack_world_core::Result<Self> {
        let world = World::load(&database).await?;

        Self::on(world, clock, Some(database)).await
    }

    /// Returns the persistent world database, if configured.
    #[must_use]
    pub fn database(&self) -> Option<&Database> {
        self.0.database.as_ref()
    }

    /// Waits for active cycles and flushes pending changes; memory-only worlds require no write.
    ///
    /// # Errors
    ///
    /// Returns a pending commit failure or the current write error.
    pub async fn save(&self) -> greyhack_world_core::Result<()> {
        self.0.cycle(async |_| (), true).await;

        match self.0.failure.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Creates a game-server channel attached to the runtime at `transport`'s peer.
    pub fn attach<D: Driver>(&self, transport: &Transport<D>) -> Channel<GameServer, D> {
        let channel = Channel::new(transport, ChannelOptions::default());
        let connection = self
            .0
            .index
            .borrow_mut()
            .add_connection(Rc::new(channel.clone()));
        let state = Rc::downgrade(&self.0);
        let spawn = connection::spawner(&channel);

        self.0.start_ticks(&spawn);
        channel.on_close(move |_| {
            if let Some(state) = state.upgrade() {
                state.end_connection(connection, &spawn);
            }
        });
        connection::attach(&self.0, connection, &channel);

        channel
    }

    /// Waits for active operations and counts objects across all connections.
    pub async fn object_count(&self) -> usize {
        let state = self.0.world.lock().await;

        state.trees.object_count()
    }

    /// Locks out other cycles and returns read-only world access. Writes outside a cycle would enter the
    /// transaction rolled back by the next cycle ([`World::begin`]); use [`WorldServer::change_world`] for changes.
    pub async fn world(&self) -> WorldView<'_> {
        WorldView(self.0.world.lock().await)
    }

    /// Clones the live world handle so a fixture callback can mutate the same database as the running server.
    pub async fn world_clone(&self) -> World {
        self.0.world.lock().await.world.as_ref().clone()
    }

    /// Runs a read-only world operation and rolls back its transaction.
    pub async fn read_world<T>(&self, read: impl AsyncFnOnce(&World) -> T) -> T {
        self.0
            .cycle(
                async |state| {
                    state
                        .world
                        .handle
                        .read_route(async || read(&state.world).await)
                        .await
                },
                false,
            )
            .await
    }

    /// Runs a world change after active operations finish. Ended program processes produce termination
    /// notifications.
    pub async fn change_world<T>(&self, change: impl AsyncFnOnce(&World) -> T) -> T {
        self.0
            .cycle(async |state| change(&state.world).await, true)
            .await
    }
}

impl ServerState {}

/// Runtime channel for program termination notifications.
pub(crate) type ConnectionChannel = Rc<dyn GameServerChannel>;
