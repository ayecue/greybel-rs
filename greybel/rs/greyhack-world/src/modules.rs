//! Composition root of every domain service.

use std::rc::Rc;

use greyhack_world_core::{CoreModule, WorldRef};
use greyhack_world_creation::CreationModule;
use greyhack_world_economy::EconomyModule;
use greyhack_world_guilds::GuildModule;
use greyhack_world_intrusion::IntrusionModule;
use greyhack_world_libraries::LibraryModule;
use greyhack_world_modules::{Assembly, Module, Registry};
use greyhack_world_npcs::NpcModule;
use greyhack_world_player::PlayerModule;
use greyhack_world_services::ServicesModule;
use greyhack_world_shell::ShellModule;
use greyhack_world_smart_devices::SmartDevicesModule;
use greyhack_world_traces::TracesModule;

/// Domain modules in dependency order.
pub struct ModuleSet {
    /// Base computers, files, and networks.
    pub core: Rc<CoreModule>,
    /// Banks, coins, wallets, and the market.
    pub economy: Rc<EconomyModule>,
    /// People, jobs, schedules, and mail.
    pub npcs: NpcModule,
    /// Guilds, companies, and CrownShield.
    pub guilds: GuildModule,
    /// Active and passive administrator traces.
    pub traces: TracesModule,
    /// Package and computer services.
    pub services: Rc<ServicesModule>,
    /// Cryptography, sniffers, and floods.
    pub intrusion: IntrusionModule,
    /// Appliances, traffic, and CCTV.
    pub smart_devices: SmartDevicesModule,
    /// Library content and exploit services.
    pub libraries: LibraryModule,
    /// Shell sessions, launches, builds, and connections.
    pub shell: ShellModule,
    /// Network and mission creation and restores.
    pub creation: CreationModule,
    /// Player accounts, CTF, missions, and game over.
    pub player: PlayerModule,
    /// Scheduling shared by the domain routine kinds.
    pub routines: Rc<crate::routines::RoutineService>,
}

/// An assembled module graph and its shared capabilities.
pub struct WorldAssembly {
    /// Domain services.
    pub modules: Rc<ModuleSet>,
    /// Schemas, scheduler, routes, events, and work.
    pub assembly: Assembly,
    /// Lazy services retained for the lifetime of the world.
    pub registry: Rc<Registry>,
}

/// Registers every module without resolving lazy service dependencies.
pub fn assemble(world: WorldRef) -> WorldAssembly {
    let registry = Rc::new(Registry::new(world.clone()));

    let core = CoreModule::register(&registry);

    registry.register_routes(greyhack_world_core::routes::register::<crate::World>);

    let economy = Rc::new(EconomyModule::register(&registry));

    registry.register_routes(greyhack_world_economy::routes::register::<crate::World>);

    let npcs = NpcModule::register(&registry);

    registry.register_routes(greyhack_world_npcs::routes::register::<crate::World>);

    let guilds = GuildModule::register(&registry);

    registry.register_routes(greyhack_world_guilds::routes::register::<crate::World>);

    let traces = TracesModule::register(&registry);

    let services = Rc::new(ServicesModule::register(&registry));

    registry.register_routes(greyhack_world_services::routes::register::<crate::World>);

    let intrusion = IntrusionModule::register(&registry);

    registry.register_routes(greyhack_world_intrusion::routes::register::<crate::World>);

    let smart_devices = SmartDevicesModule::register(&registry);

    registry.register_routes(greyhack_world_smart_devices::routes::register::<crate::World>);

    let libraries = LibraryModule::register(&registry);

    registry.register_routes(greyhack_world_libraries::routes::register::<crate::World>);

    let shell = ShellModule::register(&registry);

    registry.register_routes(greyhack_world_shell::routes::register::<crate::World>);

    let creation = CreationModule::register(&registry);

    registry.register_routes(greyhack_world_creation::routes::register::<crate::World>);
    registry.register_routes(crate::routes::register);

    let player = PlayerModule::register(&registry);

    registry.register_routes(greyhack_world_player::routes::register::<crate::World>);

    let routines = Rc::new(crate::routines::RoutineService::new(
        &world,
        Rc::new(crate::storage::RoutineRepository::new(
            world.db().clone(),
            registry.scheduler(),
        )),
        registry.scheduler(),
    ));

    registry.register_capability::<dyn greyhack_world_effects::WorkOwner>(routines.clone());
    registry.register_service(routines.clone());

    let modules = Rc::new(ModuleSet {
        core: Rc::new(core),
        economy,
        npcs,
        guilds,
        traces,
        services,
        intrusion,
        smart_devices,
        libraries,
        shell,
        creation,
        player,
        routines,
    });
    let assembly = registry.assembly();

    WorldAssembly {
        modules,
        assembly,
        registry,
    }
}
