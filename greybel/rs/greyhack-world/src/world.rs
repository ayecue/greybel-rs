//! One assembled world: its shared handle and the modules built for it.

use std::cell::{Ref, RefMut};
use std::rc::Rc;

use greyhack_world_core::{CoreContext, CoreModule, WorldRef, WorldState};
use greyhack_world_modules::Assembly;
use greyhack_world_router::WorldContext;

use crate::modules::ModuleSet;
use crate::{ComputerId, Database, Result};

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
        let parts = crate::modules::assemble(handle.clone());

        handle.attach_queues(parts.assembly.events.clone(), parts.assembly.works.clone());

        Self {
            handle,
            modules: parts.modules,
            assembly: Rc::new(parts.assembly),
            registry: parts.registry,
        }
    }

    pub(crate) fn from_core(handle: WorldRef, _core: Rc<CoreModule>) -> Self {
        let parts = crate::modules::assemble(handle.clone());

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
    pub fn world_state(&self) -> Ref<'_, WorldState> {
        self.handle.world()
    }

    /// Returns mutable shared world state for one synchronous statement or closure.
    pub fn world_state_mut(&self) -> RefMut<'_, WorldState> {
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

    /// Delivers pending domain events in publish order.
    pub fn deliver_events(&self) {
        self.assembly.events.deliver();
    }

    /// Drains computers with ended processes.
    pub fn take_changed_computers(&self) -> Vec<ComputerId> {
        self.handle.world_mut().take_changed_computers()
    }
}

impl WorldContext for World {
    type Computer = ComputerId;
}

impl greyhack_world_economy::EconomyContext for World {
    fn economy(&self) -> Rc<greyhack_world_economy::EconomyModule> {
        self.modules.economy.clone()
    }
}

impl greyhack_world_npcs::NpcContext for World {
    fn npcs(&self) -> Rc<greyhack_world_npcs::NpcModule> {
        Rc::new(self.modules.npcs.clone())
    }
}

impl greyhack_world_guilds::GuildContext for World {
    fn guilds(&self) -> Rc<greyhack_world_guilds::GuildModule> {
        Rc::new(self.modules.guilds.clone())
    }
}

impl greyhack_world_services::ServicesContext for World {
    fn services(&self) -> Rc<greyhack_world_services::ServicesModule> {
        self.modules.services.clone()
    }
}

impl greyhack_world_intrusion::IntrusionContext for World {
    fn intrusion(&self) -> Rc<greyhack_world_intrusion::IntrusionModule> {
        Rc::new(self.modules.intrusion.clone())
    }
}

impl greyhack_world_smart_devices::SmartDevicesContext for World {
    fn smart_devices(&self) -> Rc<greyhack_world_smart_devices::SmartDevicesModule> {
        Rc::new(self.modules.smart_devices.clone())
    }
}

impl greyhack_world_libraries::LibraryContext for World {
    fn libraries(&self) -> Rc<greyhack_world_libraries::LibraryModule> {
        Rc::new(self.modules.libraries.clone())
    }
}

impl greyhack_world_shell::ShellContext for World {
    fn shell(&self) -> Rc<greyhack_world_shell::ShellModule> {
        Rc::new(self.modules.shell.clone())
    }
}

impl greyhack_world_creation::CreationContext for World {
    fn creation(&self) -> Rc<greyhack_world_creation::CreationModule> {
        Rc::new(self.modules.creation.clone())
    }
}

impl greyhack_world_player::PlayerContext for World {
    fn player(&self) -> Rc<greyhack_world_player::PlayerModule> {
        Rc::new(self.modules.player.clone())
    }
}

impl CoreContext for World {
    fn world_ref(&self) -> WorldRef {
        self.handle.clone()
    }

    fn core(&self) -> Rc<CoreModule> {
        self.modules.core.clone()
    }
}

impl std::ops::Deref for World {
    type Target = WorldRef;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}
