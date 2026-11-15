#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack intrusion domain: file and Wi-Fi cryptography, path sniffers and captured
//! packets, and script-driven floods and DDoS attacks.
//!
//! [`IntrusionModule`] registers the domain schema, services, typed-work ownership, event
//! subscribers, and NPC capability implementations. The composition root registers
//! [`routes::register`] for its concrete world and the libraries domain's
//! [`LibraryObjectAccess`] implementation. Public entities, object handles, and services are
//! re-exported at the crate root; repositories remain private to the domain.

pub mod model;
pub mod objects;
pub mod routes;
pub mod services;
pub mod storage;

use std::rc::Rc;

use greyhack_world_core::WorldRef;
use greyhack_world_modules::{Dep, Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use model::{SnifferPacket, WifiCapture};
pub use objects::{CryptoHandle, ObjectTableExt};
pub use services::capture::AckCount;
pub use services::crypto::{CryptoFailure, WifiCaptureRun};
pub use services::ddos::{FloodAttempt, FloodWarning};
pub use services::flood::Flood;
pub use services::sniffer::encoder::EncoderFailure;
pub use services::{Crypto, FloodService, LibraryObjectAccess, ObjectAccessFuture, SnifferService};
use storage::SubscriberRepository;

/// Services, typed-work ownership, and subscriber storage of the intrusion domain.
#[derive(Clone)]
pub struct IntrusionModule {
    /// File encryption, monitor mode, Wi-Fi capture, and key recovery.
    pub crypto: Rc<Crypto>,
    /// Sniffer paths, captured packets, NPC logins, and encoders.
    pub sniffers: Rc<SnifferService>,
    /// Floods and DDoS rules.
    pub floods: Rc<FloodService>,
    pub(crate) library_objects: Dep<dyn LibraryObjectAccess>,
}

impl IntrusionModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let subscribers = Rc::new(SubscriberRepository::new(world.db().clone()));
        let sniffers = Rc::new(SnifferService::new(world, subscribers.clone(), registry));
        let floods = Rc::new(FloodService::new(
            world,
            subscribers,
            registry.get(),
            registry.get(),
            registry.get(),
            registry.get(),
        ));

        Self {
            crypto: Rc::new(Crypto::new(world, registry.get(), registry.get())),
            sniffers,
            floods,
            library_objects: registry.get(),
        }
    }

    /// Returns the intrusion schema in creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(
            <Self as Module>::NAME,
            [storage::schema::GROUP, storage::schema::VOLATILE],
        )
        .with_dependencies(&["core"])
    }
}

impl Module for IntrusionModule {
    const NAME: &'static str = "intrusion";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);

        registry
            .register_service::<dyn greyhack_world_npcs::NpcLoginCapture>(module.sniffers.clone());
        registry.register_service::<dyn greyhack_world_npcs::SmtpAccess>(module.crypto.clone());
        registry.register_async_subscriber({
            let sniffers = module.sniffers.clone();

            move |event: &greyhack_world_guilds::FloodClear| {
                let sniffers = sniffers.clone();

                Box::pin(async move {
                    sniffers
                        .subscribers
                        .clear_floods(event.computer)
                        .await
                        .map_err(greyhack_world_effects::EventError::new)
                })
            }
        });
        registry.register_async_subscriber({
            let sniffers = module.sniffers.clone();

            move |event: &greyhack_world_core::LoginPacket| {
                let sniffers = sniffers.clone();

                Box::pin(async move {
                    sniffers
                        .record_packet(
                            event.source,
                            event.destination,
                            &event.address,
                            event.port,
                            &event.user,
                            &event.password,
                        )
                        .await
                        .map_err(greyhack_world_effects::EventError::new)
                })
            }
        });
        registry
            .register_capability::<dyn greyhack_world_effects::WorkOwner>(module.sniffers.clone());
        registry
            .register_capability::<dyn greyhack_world_effects::WorkOwner>(module.floods.clone());
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.crypto.clone());
        registry.register_service(module.sniffers.clone());
        registry.register_service(module.floods.clone());

        module
    }
}
