//! Cheap shared access to one world's connection and interior-mutable state.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;

use greyhack_behaviour::format_date;

use crate::{ComputerId, Database, PersonId, World};

struct WorldCell {
    db: Database,
    world: RefCell<World>,
    read_only: Cell<bool>,
}

/// Shared handle to one world. Services clone this handle rather than borrowing the world.
#[derive(Clone)]
pub struct WorldRef(Rc<WorldCell>);

impl std::fmt::Debug for WorldRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WorldRef")
    }
}

impl WorldRef {
    /// Creates a world on `db` with its initial shared state.
    #[must_use]
    pub fn on(db: Database, seed: u64, player: ComputerId, time_seconds: u64) -> Self {
        Self(Rc::new(WorldCell {
            db,
            world: RefCell::new(World::new(seed, player, time_seconds)),
            read_only: Cell::new(false),
        }))
    }

    /// Creates a handle around existing shared `world` state.
    #[must_use]
    pub fn from_world(db: Database, world: World) -> Self {
        Self(Rc::new(WorldCell {
            db,
            world: RefCell::new(world),
            read_only: Cell::new(false),
        }))
    }

    /// Borrows shared world state for one synchronous statement or closure.
    pub fn world(&self) -> Ref<'_, World> {
        self.0.world.borrow()
    }

    /// Mutably borrows shared world state for one synchronous statement or closure.
    ///
    /// # Panics
    ///
    /// Panics when called by a route registered as read-only.
    pub fn world_mut(&self) -> RefMut<'_, World> {
        assert!(
            !self.0.read_only.get(),
            "a read route attempted to change the world"
        );

        self.0.world.borrow_mut()
    }

    /// Returns the ordered event queue.
    #[must_use]
    pub fn events(&self) -> Rc<greyhack_world_effects::EventQueue> {
        self.world().events.clone()
    }

    /// Returns the typed work queue.
    #[must_use]
    pub fn works(&self) -> Rc<greyhack_world_effects::WorkQueue> {
        self.world().works.clone()
    }

    /// Attaches the event and work queues created by module assembly.
    pub fn attach_queues(
        &self,
        events: Rc<greyhack_world_effects::EventQueue>,
        works: Rc<greyhack_world_effects::WorkQueue>,
    ) {
        self.world_mut().attach_queues(events, works);
    }

    /// Returns the connection-only database handle.
    #[must_use]
    pub fn db(&self) -> &Database {
        &self.0.db
    }

    /// Runs `read` while mutable world access is refused. The server rolls its transaction back.
    pub async fn read_route<T>(&self, read: impl AsyncFnOnce() -> T) -> T {
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        assert!(!self.0.read_only.replace(true), "nested read routes");

        let reset = Reset(&self.0.read_only);
        let result = read().await;

        drop(reset);

        result
    }

    /// Returns the world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.world().seed
    }

    /// Returns the player computer.
    #[must_use]
    pub fn player(&self) -> ComputerId {
        self.world().player
    }

    /// Returns the player's person.
    #[must_use]
    pub fn player_person(&self) -> PersonId {
        self.world().player_person
    }

    /// Returns fractional clock carry.
    #[must_use]
    pub fn clock_carry(&self) -> u32 {
        self.world().clock_carry
    }

    /// Returns whether the player has a computer.
    #[must_use]
    pub fn has_player(&self) -> bool {
        self.world().has_player()
    }

    /// Returns whether `computer` belongs to the player.
    #[must_use]
    pub fn is_player(&self, computer: ComputerId) -> bool {
        self.world().is_player(computer)
    }

    /// Returns the date displayed by `current_date`.
    #[must_use]
    pub fn date(&self) -> String {
        format_date(self.time_seconds())
    }

    /// Returns the world clock in seconds.
    #[must_use]
    pub fn time_seconds(&self) -> u64 {
        self.world().time_seconds
    }

    /// Returns the world secret (`WorldSeed`).
    #[must_use]
    pub fn world_seed(&self) -> greyhack_behaviour::WorldSeed {
        self.world().world_seed.clone()
    }

    /// Replaces the world secret from mirrored Grey Hack state.
    pub fn set_world_secret(&self, secret: Vec<u8>) {
        self.world_mut().set_world_secret(secret);
    }

    /// Begins a database transaction.
    pub async fn begin(&self) -> crate::Result<()> {
        Ok(self.db().begin().await?)
    }

    /// Rolls back the open transaction. In-memory state is unchanged.
    pub async fn rollback(&self) -> crate::Result<()> {
        Ok(self.db().rollback().await?)
    }
}
