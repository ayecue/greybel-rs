//! Registration of the base world module.

use greyhack_world_modules::{Module, Registry};
use greyhack_world_routines::QUEUE_TABLES;
use greyhack_world_storage::DomainSchema;

use crate::CoreModule;
use crate::storage::schema::{CORE_TABLES, WORLD_META_TABLES};

impl Module for CoreModule {
    const NAME: &'static str = "core";

    fn register(registry: &std::rc::Rc<Registry>) -> Self {
        let core = CoreModule::new(registry.world(), registry);

        registry.register_schema(DomainSchema::new(
            Self::NAME,
            CORE_TABLES
                .into_iter()
                .chain([QUEUE_TABLES, WORLD_META_TABLES]),
        ));
        registry.register_service(core.clone());
        registry.register_service(core.computers.clone());
        registry.register_service(core.files.clone());
        registry.register_service(core.networks.clone());

        (*core).clone()
    }
}

/// Base schema registration retained for callers that only open core tables.
pub struct CoreDomain;

impl CoreDomain {
    /// Returns the core schema without assembling services.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <CoreModule as Module>::NAME,
            CORE_TABLES
                .into_iter()
                .chain([QUEUE_TABLES, WORLD_META_TABLES]),
        )
    }
}
