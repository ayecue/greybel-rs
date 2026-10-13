//! Game server for one shared world. Operations, ticks, saves, and connection cleanup run as cycles.
//! Cycles acquire the world in request order, apply their changes, and process resulting events in fixed
//! phases before releasing the world.

mod budget;
mod cycle;
mod deferred_jobs;
mod lifecycle;
mod local;
mod programs;
mod programs_view;
mod ticks;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use budget::BudgetTable;
use deferred_jobs::DeferredJob;
use duplex_core::{Channel, ChannelOptions, Driver, Transport};
use greyhack_world::{Database, WorkEnd, World, WorldError};
use programs::ProgramTable;
pub(crate) use programs::TerminalInfo;
use scheduler::Clock;
pub use ticks::TickOptions;
use ticks::TickState;
use tokio::sync::{Mutex, MutexGuard};

use crate::connection::objects::ObjectTable;
use crate::connection::{self};
use crate::protocol::{GameServer, GameServerChannel, ProgramEnd};

/// Shared-world server accepting runtime channels that execute operations on the same world.
#[derive(Clone)]
pub struct WorldServer(Rc<ServerState>);

pub(crate) struct ServerState {
    /// World and connection object tables locked for an entire cycle, including awaits. Lock waiters are
    /// served in request order.
    world: Mutex<WorldState>,
    programs: RefCell<ProgramTable>,
    /// Optional persistent world database.
    database: Option<Database>,
    /// Deferred database error returned by the next [`WorldServer::save`].
    failure: RefCell<Option<WorldError>>,
    /// Host clock used to schedule ticks.
    clock: Rc<dyn Clock>,
    ticks: TickState,
    /// Program-tree budgets and active world claims.
    budgets: BudgetTable,
    /// Program termination notices queued for the end of the cycle, after operation responses.
    program_ends: RefCell<Vec<(ProgramEnd, ConnectionChannel)>>,
    /// Deferred operations keyed by scheduled work ID.
    deferred_jobs: RefCell<HashMap<u64, DeferredJob>>,
    /// Work results stored until a successful commit resolves their deferred jobs.
    pending_ends: RefCell<Vec<WorkEnd>>,
}

/// World and connection object tables the current cycle exclusively owns.
pub(crate) struct WorldState {
    pub(crate) world: World,
    objects: HashMap<u32, ObjectTable>,
}

impl WorldState {
    /// Returns mutable world access and the object table for `connection`.
    pub(crate) fn of(&mut self, connection: u32) -> (&mut World, &mut ObjectTable) {
        (&mut self.world, self.objects.entry(connection).or_default())
    }
}

impl WorldServer {
    /// Creates a server for `world` using `clock`. Generated worlds use their in-memory database, so
    /// [`WorldServer::save`] reports that nothing was saved.
    #[must_use]
    pub fn new(world: World, clock: Rc<dyn Clock>) -> Self {
        Self::on(world, clock, None)
    }

    fn on(world: World, clock: Rc<dyn Clock>, database: Option<Database>) -> Self {
        Self(Rc::new(ServerState {
            database,
            world: Mutex::new(WorldState {
                world,
                objects: HashMap::new(),
            }),
            programs: RefCell::new(ProgramTable::default()),
            failure: RefCell::new(None),
            clock,
            ticks: TickState::default(),
            budgets: BudgetTable::default(),
            program_ends: RefCell::new(Vec::new()),
            deferred_jobs: RefCell::new(HashMap::new()),
            pending_ends: RefCell::new(Vec::new()),
        }))
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
        mut world: World,
        database: Database,
        clock: Rc<dyn Clock>,
    ) -> greyhack_world::Result<Self> {
        world.save_into(&database).await?;

        Ok(Self::on(
            World::load(&database).await?,
            clock,
            Some(database),
        ))
    }

    /// Loads a world from `database` and persists later changes while using `clock`.
    ///
    /// # Errors
    ///
    /// Returns world-loading errors.
    pub async fn open(database: Database, clock: Rc<dyn Clock>) -> greyhack_world::Result<Self> {
        let world = World::load(&database).await?;

        Ok(Self::on(world, clock, Some(database)))
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
    pub async fn save(&self) -> greyhack_world::Result<()> {
        self.0.cycle(async |_| ()).await;

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
            .programs
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
        self.0
            .world
            .lock()
            .await
            .objects
            .values()
            .map(ObjectTable::len)
            .sum()
    }

    /// Locks out other cycles and returns read-only world access. Writes outside a cycle would enter the
    /// transaction rolled back by the next cycle ([`World::begin`]); use [`WorldServer::change_world`] for changes.
    pub async fn world(&self) -> impl std::ops::Deref<Target = World> + '_ {
        MutexGuard::map(self.0.world.lock().await, |state| &mut state.world)
    }

    /// Runs a world change after active operations finish. Ended program processes produce termination
    /// notifications.
    pub async fn change_world<T>(&self, change: impl AsyncFnOnce(&mut World) -> T) -> T {
        self.0
            .cycle(async |state| change(&mut state.world).await)
            .await
    }
}

impl ServerState {}

/// Runtime channel for program termination notifications.
pub(crate) type ConnectionChannel = Rc<dyn GameServerChannel>;
