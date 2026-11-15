#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack shell domain: source builds and imports, remote service connections, child
//! programs and terminals, and the process, trace, and log effects of ending those programs.
//!
//! [`ShellModule`] registers the domain's empty schema and services. The composition root
//! registers [`routes::register`] for its concrete world. Program trees remain in the router and
//! programs crates; shell routes access them through the route context. Public entities and
//! services are re-exported at the crate root.

pub mod routes;
pub mod services;

use std::rc::Rc;

use greyhack_world_core::WorldRef;
pub use greyhack_world_core::{
    ComputerId, FileFailure, LoginFailure, ProcessFailure, Result, WorldError,
};
use greyhack_world_modules::{Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use services::build::{BuildFailure, BuildService, BuiltSource, ImportFailure};
pub use services::connect::{
    ConnectionService, Encryption, EncryptionScript, ServiceOpen, ServiceRequest,
};
pub use services::launch::{
    Launch, LaunchFailure, LaunchService, LaunchedProgram, TerminalFailure, TerminalProgram,
};
pub use services::program_end::ShellService;
pub use services::{build, connect, launch, program_end};

/// Shell services built from core, services, guilds, intrusion, and libraries.
#[derive(Clone)]
pub struct ShellModule {
    /// Program-end processes, traces, and terminal logs.
    pub shells: Rc<ShellService>,
    /// Child programs and terminal startup.
    pub launches: Rc<LaunchService>,
    /// Source builds, imports, binary writes, and shell copies.
    pub builds: Rc<BuildService>,
    /// Remote service logins and encrypted handshakes.
    pub connections: Rc<ConnectionService>,
}

impl ShellModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();

        Self {
            shells: Rc::new(ShellService::new(
                registry.get(),
                registry.get(),
                registry.get(),
            )),
            launches: Rc::new(LaunchService::new(
                world,
                registry.get(),
                registry.get(),
                registry.get(),
            )),
            builds: Rc::new(BuildService::new(
                world,
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
            )),
            connections: Rc::new(ConnectionService::new(
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
                registry.get(),
            )),
        }
    }

    /// Returns the shell schema, which owns no tables.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(<Self as Module>::NAME, [])
    }
}

impl Module for ShellModule {
    const NAME: &'static str = "shell";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.shells.clone());
        registry.register_service(module.launches.clone());
        registry.register_service(module.builds.clone());
        registry.register_service(module.connections.clone());

        module
    }
}
