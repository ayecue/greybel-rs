use std::rc::Rc;

use greyhack_behaviour::Rng;
use greyhack_world_effects::QUEUE_TABLES;
use greyhack_world_modules::{Dep, Module, Registry};
use greyhack_world_storage::DomainSchema;

use crate::storage::schema::{CORE_TABLES, WORLD_META_TABLES};
use crate::storage::world_meta::WorldMeta;
use crate::storage::{CoreStorage, Database, WorldMetaRepository};
use crate::systems::computer::hardware::Hardware;
use crate::systems::{
    ComputerService, Dns, FileChangeService, FileLookup, FilePermissionService, FileSizeService,
    FileSystemService, LibraryFileService, LogService, NetCardService, NetworkConnect, NetworkLan,
    NetworkReach, NetworkService, NetworkSettings, NetworkTopology, ProcessService, RouterConfig,
    RouterService, ScriptFileService, Startup, UserService,
};
use crate::{Result, World, WorldRef};

struct FoundationTable {
    users: Rc<UserService>,
    lookup: Rc<FileLookup>,
    library_files: Rc<LibraryFileService>,
    permissions: Rc<FilePermissionService>,
    sizes: Rc<FileSizeService>,
    changes: Rc<FileChangeService>,
}

impl FoundationTable {
    fn new(registry: &Rc<Registry>, repositories: &CoreStorage) -> Self {
        Self {
            users: Rc::new(UserService::new(
                registry,
                repositories.computers.clone(),
                repositories.users.clone(),
            )),
            lookup: Rc::new(FileLookup::new(registry, repositories.nodes.clone())),
            library_files: Rc::new(LibraryFileService::new(
                registry,
                repositories.nodes.clone(),
                repositories.libraries.clone(),
            )),
            permissions: Rc::new(FilePermissionService::new(
                registry,
                repositories.nodes.clone(),
            )),
            sizes: Rc::new(FileSizeService::new(
                registry,
                repositories.nodes.clone(),
                repositories.hardware.clone(),
            )),
            changes: Rc::new(FileChangeService::new(
                registry,
                repositories.nodes.clone(),
                repositories.libraries.clone(),
                repositories.computers.clone(),
            )),
        }
    }
}

struct DeviceTable {
    lan: Rc<NetworkLan>,
    topology: Rc<NetworkTopology>,
    logs: Rc<LogService>,
    scripts: Rc<ScriptFileService>,
    startup: Rc<Startup>,
    hardware: Rc<Hardware>,
}

impl DeviceTable {
    fn new(registry: &Rc<Registry>, repositories: &CoreStorage) -> Self {
        Self {
            lan: Rc::new(NetworkLan::new(
                registry,
                repositories.computers.clone(),
                repositories.routers.clone(),
            )),
            topology: Rc::new(NetworkTopology::new(
                registry,
                repositories.computers.clone(),
                repositories.routers.clone(),
            )),
            logs: Rc::new(LogService::new(registry)),
            scripts: Rc::new(ScriptFileService::new(
                registry,
                repositories.computers.clone(),
            )),
            startup: Rc::new(Startup::new(registry, repositories.computers.clone())),
            hardware: Rc::new(Hardware::new(
                registry,
                repositories.computers.clone(),
                repositories.hardware.clone(),
                repositories.nodes.clone(),
                repositories.users.clone(),
                repositories.libraries.clone(),
            )),
        }
    }
}

struct RouterTable {
    settings: Rc<NetworkSettings>,
    router_config: Rc<RouterConfig>,
    reach: Rc<NetworkReach>,
    routers: Rc<RouterService>,
    dns: Rc<Dns>,
    connect: Rc<NetworkConnect>,
    net_cards: Rc<NetCardService>,
}

impl RouterTable {
    fn new(registry: &Rc<Registry>, repositories: &CoreStorage) -> Self {
        Self {
            settings: Rc::new(NetworkSettings::new(
                repositories.computers.clone(),
                repositories.routers.clone(),
            )),
            router_config: Rc::new(RouterConfig::new(repositories.routers.clone())),
            reach: Rc::new(NetworkReach::new(
                registry,
                repositories.computers.clone(),
                repositories.routers.clone(),
                repositories.hardware.clone(),
            )),
            routers: Rc::new(RouterService::new(
                registry,
                repositories.computers.clone(),
                repositories.routers.clone(),
            )),
            dns: Rc::new(Dns::new(registry, repositories.routers.clone())),
            connect: Rc::new(NetworkConnect::new(
                registry,
                repositories.computers.clone(),
                repositories.routers.clone(),
            )),
            net_cards: Rc::new(NetCardService::new(
                registry,
                repositories.computers.clone(),
                repositories.hardware.clone(),
                repositories.routers.clone(),
            )),
        }
    }
}

struct CoreTable {
    processes: Rc<ProcessService>,
    computers: Rc<ComputerService>,
    files: Rc<FileSystemService>,
    networks: Rc<NetworkService>,
}

impl CoreTable {
    fn new(registry: &Rc<Registry>, repositories: &CoreStorage) -> Self {
        Self {
            processes: Rc::new(ProcessService::new(
                registry,
                repositories.computers.clone(),
                repositories.hardware.clone(),
                repositories.libraries.clone(),
                repositories.users.clone(),
                repositories.persons.clone(),
            )),
            computers: Rc::new(ComputerService::new(
                registry,
                repositories.computers.clone(),
                repositories.hardware.clone(),
                repositories.users.clone(),
                repositories.persons.clone(),
            )),
            files: Rc::new(FileSystemService::new(
                registry,
                repositories.nodes.clone(),
                repositories.libraries.clone(),
            )),
            networks: Rc::new(NetworkService::new(registry, repositories.routers.clone())),
        }
    }
}

/// Core services and the upward calls their rules make.
#[derive(Clone)]
pub struct CoreModule {
    /// Shared world handle.
    pub world: WorldRef,
    world_meta_rows: Rc<WorldMetaRepository>,
    /// Computer services.
    pub computers: Rc<ComputerService>,
    /// File-system services.
    pub files: Rc<FileSystemService>,
    /// Network services.
    pub networks: Rc<NetworkService>,
    /// Load/save capabilities contributed by all modules.
    pub persistent_state: Dep<greyhack_world_modules::CapabilityRegistry>,
}

impl CoreModule {
    /// Returns the core schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            CORE_TABLES
                .into_iter()
                .chain([QUEUE_TABLES, WORLD_META_TABLES]),
        )
    }

    /// Builds every core service once for `world` using the caller's registry.
    #[must_use]
    pub fn new(world: &WorldRef, registry: &Rc<greyhack_world_modules::Registry>) -> Rc<Self> {
        registry.register_service(registry.capabilities());

        let repositories = CoreStorage::new(world.db().clone());
        let foundation = FoundationTable::new(registry, &repositories);
        let devices = DeviceTable::new(registry, &repositories);
        let router = RouterTable::new(registry, &repositories);
        let core = CoreTable::new(registry, &repositories);

        registry.register_service(foundation.users.clone());
        registry.register_service(foundation.lookup.clone());
        registry.register_service(foundation.library_files.clone());
        registry.register_service(foundation.permissions.clone());
        registry.register_service(foundation.sizes.clone());
        registry.register_service(foundation.changes.clone());
        registry.register_service(devices.lan.clone());
        registry.register_service(devices.topology.clone());
        registry.register_service(devices.logs.clone());
        registry.register_service(devices.scripts.clone());
        registry.register_service(devices.startup.clone());
        registry.register_service(devices.hardware.clone());
        registry.register_service(router.settings.clone());
        registry.register_service(router.router_config.clone());
        registry.register_service(router.reach.clone());
        registry.register_service(router.routers.clone());
        registry.register_service(router.dns.clone());
        registry.register_service(router.connect.clone());
        registry.register_service(router.net_cards.clone());
        registry.register_service(core.processes.clone());

        Rc::new(Self {
            world: world.clone(),
            world_meta_rows: repositories.world_meta.clone(),
            computers: core.computers,
            files: core.files,
            networks: core.networks,
            persistent_state: registry.get(),
        })
    }

    /// Loads shared state and builds core services; the caller retains the returned registry.
    pub async fn load(
        db: &Database,
    ) -> Result<(WorldRef, Rc<Self>, Rc<greyhack_world_modules::Registry>)> {
        let meta = WorldMetaRepository::new(db.clone())
            .world_meta()
            .await?
            .ok_or_else(|| {
                turso::Error::ConversionFailure("the database has no world".to_owned())
            })?;
        let mut world = World::new(meta.seed, meta.player, meta.elapsed_seconds);

        world.rng = Rng::new(meta.rng_state);
        world.player_person = meta.player_person;
        world.clock_carry = meta.clock_carry;
        world.world_seed = greyhack_behaviour::WorldSeed::new(meta.world_secret);

        let world = WorldRef::from_world(db.clone(), world);
        let registry = Rc::new(greyhack_world_modules::Registry::new(world.clone()));
        let core = Self::new(&world, &registry);

        Ok((world, core, registry))
    }

    /// Writes shared metadata and domain state, then commits the open transaction.
    pub async fn commit(&self) -> Result<()> {
        let own = !self.world.db().in_transaction()?;

        if own {
            self.world.db().begin().await?;
        }

        let commit = async {
            let meta = {
                let world = self.world.world();

                WorldMeta {
                    seed: world.seed,
                    rng_state: world.rng.state(),
                    player: world.player,
                    player_person: world.player_person,
                    elapsed_seconds: world.time_seconds,
                    clock_carry: world.clock_carry,
                    world_secret: world.world_seed.secret().to_vec(),
                }
            };

            self.world_meta_rows.save_world_meta(&meta).await?;

            for capability in self.persistent_state.all::<dyn crate::PersistentState>() {
                capability.save_state(&self.world).await?;
            }

            self.world.db().commit().await?;

            Ok(())
        }
        .await;

        if commit.is_err() {
            self.world.db().rollback().await?;
        }

        commit
    }

    /// Copies this world into `db` after flushing its current state.
    pub async fn save_into(&self, db: &Database) -> Result<()> {
        self.commit().await?;

        Ok(self.world.db().copy_into(db).await?)
    }

    /// Loads state through every module's registered persistence capability.
    pub async fn load_state(&self) -> Result<()> {
        for capability in self.persistent_state.all::<dyn crate::PersistentState>() {
            capability.load_state(&self.world).await?;
        }

        Ok(())
    }
}

impl Module for CoreModule {
    const NAME: &'static str = "core";

    fn register(registry: &Rc<Registry>) -> Self {
        let core = CoreModule::new(registry.world(), registry);

        registry.register_schema(Self::schema());
        registry.register_service(core.clone());
        registry.register_service(core.computers.clone());
        registry.register_service(core.files.clone());
        registry.register_service(core.networks.clone());

        (*core).clone()
    }
}
