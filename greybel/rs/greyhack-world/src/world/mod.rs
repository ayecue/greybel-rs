//! One assembled world: its shared handle, constructors, and the modules built for it.

mod construction;

use std::cell::{Ref, RefMut};
use std::rc::Rc;

use greyhack_world_core::{ComputerId, CoreModule, Database, Result, WorldRef};
use greyhack_world_modules::Assembly;
use greyhack_world_router::WorldContext;

use crate::modules::ModuleSet;

/// A world and every service built once for it.
#[derive(Clone)]
pub struct World {
    /// Shared connection and in-memory state.
    pub handle: WorldRef,
    /// Domain modules in dependency order.
    pub modules: Rc<ModuleSet>,
    /// Shared assembled capabilities.
    pub assembly: Rc<Assembly>,
    /// Lazy service registry retained by every `Dep`.
    pub registry: Rc<greyhack_world_modules::Registry>,
}

impl World {
    /// Creates an assembled world on `db` with newly generated shared state.
    #[must_use]
    pub fn on(db: Database, seed: u64, player: ComputerId, time_seconds: u64) -> Self {
        let handle = WorldRef::on(db, seed, player, time_seconds);
        let parts = crate::modules::assemble(&handle.clone());

        handle.attach_queues(parts.assembly.events.clone(), parts.assembly.works.clone());

        Self {
            handle,
            modules: parts.modules,
            assembly: Rc::new(parts.assembly),
            registry: parts.registry,
        }
    }

    pub(crate) fn from_core(handle: WorldRef, _core: Rc<CoreModule>) -> Self {
        let parts = crate::modules::assemble(&handle.clone());

        handle.attach_queues(parts.assembly.events.clone(), parts.assembly.works.clone());

        Self {
            handle,
            modules: parts.modules,
            assembly: Rc::new(parts.assembly),
            registry: parts.registry,
        }
    }

    /// Begins the cycle transaction.
    pub async fn begin(&self) -> Result<()> {
        self.handle.begin().await
    }

    /// Writes world metadata and commits the cycle transaction.
    pub async fn commit(&self) -> Result<()> {
        self.modules.core.commit().await
    }

    /// Rolls back the cycle transaction.
    pub async fn rollback(&self) -> Result<()> {
        self.handle.rollback().await
    }

    /// Copies this world into `db` after flushing its state.
    pub async fn save_into(&self, db: &Database) -> Result<()> {
        self.modules.core.save_into(db).await
    }

    /// Returns the world database connection.
    #[must_use]
    pub fn database(&self) -> &Database {
        self.handle.db()
    }

    /// Returns shared world state for one synchronous statement or closure.
    pub fn world_state(&self) -> Ref<'_, greyhack_world_core::World> {
        self.handle.world()
    }

    /// Returns mutable shared world state for one synchronous statement or closure.
    pub fn world_state_mut(&self) -> RefMut<'_, greyhack_world_core::World> {
        self.handle.world_mut()
    }

    /// Returns domain state `T`, creating its default value on first access.
    pub fn state<T: Default + Send + 'static>(&self) -> Ref<'_, T> {
        self.handle.world_mut().state_mut::<T>();

        Ref::map(self.handle.world(), |world| {
            world.state_ref::<T>().expect("state was initialized")
        })
    }

    /// Returns mutable domain state `T`, creating its default value on first access.
    pub fn state_mut<T: Default + Send + 'static>(&self) -> RefMut<'_, T> {
        RefMut::map(self.handle.world_mut(), |world| world.state_mut::<T>())
    }

    /// Returns domain state `T` when it has been created.
    pub fn state_ref<T: Send + 'static>(&self) -> Option<Ref<'_, T>> {
        let world = self.handle.world();

        if world.state_ref::<T>().is_some() {
            Some(Ref::map(world, |world| {
                world.state_ref::<T>().expect("state was checked")
            }))
        } else {
            None
        }
    }

    /// Dispatches pending domain events to their subscribers in publish order.
    pub fn dispatch_events(&self) {
        self.assembly.events.dispatch();
    }

    /// Drains computers with ended processes.
    pub fn take_changed_computers(&self) -> Vec<ComputerId> {
        self.handle.world_mut().take_changed_computers()
    }
}

impl WorldContext for World {
    type Computer = ComputerId;

    fn module<M: 'static>(&self) -> Rc<M> {
        self.registry
            .resolve::<M>()
            .unwrap_or_else(|| panic!("`{}` is not registered", std::any::type_name::<M>()))
    }
}

impl std::ops::Deref for World {
    type Target = WorldRef;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
