use std::rc::Rc;

use crate::storage::world_meta::WorldMeta;
use crate::storage::*;
use crate::systems::computer::hardware::Hardware;
use crate::systems::*;
use crate::*;

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
    /// Builds every core service once for `world` using the caller's registry.
    #[must_use]
    pub fn new(world: &WorldRef, registry: &Rc<greyhack_world_modules::Registry>) -> Rc<Self> {
        registry.register_service(registry.capabilities());

        let repositories = CoreStorage::new(world.db().clone());
        let computer_rows = repositories.computers.clone();
        let hardware_rows = repositories.hardware.clone();
        let node_rows = repositories.nodes.clone();
        let user_rows = repositories.users.clone();
        let router_rows = repositories.routers.clone();
        let library_rows = repositories.libraries.clone();
        let person_rows = repositories.persons.clone();
        let world_meta_rows = repositories.world_meta.clone();

        let users = Rc::new(UserService::new(
            registry,
            computer_rows.clone(),
            user_rows.clone(),
        ));

        let lookup = Rc::new(FileLookup::new(registry, node_rows.clone()));

        let library_files = Rc::new(LibraryFileService::new(
            registry,
            node_rows.clone(),
            library_rows.clone(),
        ));

        let permissions = Rc::new(FilePermissionService::new(registry, node_rows.clone()));

        let sizes = Rc::new(FileSizeService::new(
            registry,
            node_rows.clone(),
            hardware_rows.clone(),
        ));

        let changes = Rc::new(FileChangeService::new(
            registry,
            node_rows.clone(),
            library_rows.clone(),
            computer_rows.clone(),
        ));

        let lan = Rc::new(NetworkLan::new(
            registry,
            computer_rows.clone(),
            router_rows.clone(),
        ));

        let topology = Rc::new(NetworkTopology::new(
            registry,
            computer_rows.clone(),
            router_rows.clone(),
        ));

        let logs = Rc::new(LogService::new(registry));

        let scripts = Rc::new(ScriptFileService::new(registry, computer_rows.clone()));

        let startup = Rc::new(Startup::new(registry, computer_rows.clone()));

        let hardware = Rc::new(Hardware::new(
            registry,
            computer_rows.clone(),
            hardware_rows.clone(),
            node_rows.clone(),
            user_rows.clone(),
            library_rows.clone(),
        ));

        let settings = Rc::new(NetworkSettings::new(
            computer_rows.clone(),
            router_rows.clone(),
        ));

        let router_config = Rc::new(RouterConfig::new(router_rows.clone()));

        let reach = Rc::new(NetworkReach::new(
            registry,
            computer_rows.clone(),
            router_rows.clone(),
            hardware_rows.clone(),
        ));

        let routers = Rc::new(RouterService::new(
            registry,
            computer_rows.clone(),
            router_rows.clone(),
        ));

        let dns = Rc::new(Dns::new(registry, router_rows.clone()));

        let connect = Rc::new(NetworkConnect::new(
            registry,
            computer_rows.clone(),
            router_rows.clone(),
        ));

        let net_cards = Rc::new(NetCardService::new(
            registry,
            computer_rows.clone(),
            hardware_rows.clone(),
            router_rows.clone(),
        ));

        let processes = Rc::new(ProcessService::new(
            registry,
            computer_rows.clone(),
            hardware_rows.clone(),
            library_rows.clone(),
            user_rows.clone(),
            person_rows.clone(),
        ));

        let computers = Rc::new(ComputerService::new(
            registry,
            computer_rows.clone(),
            hardware_rows.clone(),
            user_rows.clone(),
            person_rows.clone(),
        ));

        let files = Rc::new(FileSystemService::new(
            registry,
            node_rows.clone(),
            library_rows.clone(),
        ));

        let networks = Rc::new(NetworkService::new(registry, router_rows.clone()));

        registry.register_service::<dyn AccountFileService>(files.clone());
        registry.register_service(users.clone());
        registry.register_service(lookup.clone());
        registry.register_service(library_files.clone());
        registry.register_service(permissions.clone());
        registry.register_service(sizes.clone());
        registry.register_service(changes.clone());
        registry.register_service(lan.clone());
        registry.register_service(topology.clone());
        registry.register_service(logs.clone());
        registry.register_service(scripts.clone());
        registry.register_service(startup.clone());
        registry.register_service(hardware.clone());
        registry.register_service(settings.clone());
        registry.register_service(router_config.clone());
        registry.register_service(reach.clone());
        registry.register_service(routers.clone());
        registry.register_service(dns.clone());
        registry.register_service(connect.clone());
        registry.register_service(net_cards.clone());
        registry.register_service(processes.clone());

        Rc::new(Self {
            world: world.clone(),
            world_meta_rows,
            computers,
            files,
            networks,
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
