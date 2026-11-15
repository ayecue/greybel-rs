#![deny(clippy::await_holding_refcell_ref)]

//! The Grey Hack player domain: account and network lookups, directory and library loading,
//! capture-the-flag accounts and events, procedural-mission offers, and third-strike game over.
//!
//! [`PlayerModule`] registers the CTF schema, services, and the guild-strike subscriber. The
//! composition root registers [`routes::register`] for its concrete world. Public entities,
//! object handles, failures, and services are re-exported at the crate root; the CTF repository
//! remains private to the domain.

pub mod model;
pub mod objects;
pub mod routes;
pub mod services;
pub mod storage;

use std::rc::Rc;

use greyhack_world_core::WorldRef;
use greyhack_world_modules::{Module, Registry};
use greyhack_world_storage::DomainSchema;
pub use model::{CtfAccount, CtfEvent, CtfFailure, CtfPlayer};
pub use objects::{CtfHandle, CtfRef, ObjectTableExt};
pub use services::{
    AcceptedMission, Ctf, CtfPasswordFailure, FolderChange, FolderLookup, GameOver, GameOverEvent,
    IncludedLibrary, MissionFailure, MissionOfferService, PlayerAccount, PlayerService, ctf,
    game_over, lookups, missions,
};
use storage::CtfRepository;

/// Player-facing services built from core and every lower domain they use.
#[derive(Clone)]
pub struct PlayerModule {
    /// Player account, router, folder, library, mail, and CTF lookups.
    pub player: Rc<PlayerService>,
    /// CTF accounts, events, participation, completion, and ratings.
    pub ctf: Rc<Ctf>,
    /// Procedural-mission acceptance and contract delivery.
    pub mission_offers: Rc<MissionOfferService>,
    /// Third-strike removal of the player and their world state.
    pub game_over: Rc<GameOver>,
}

impl PlayerModule {
    fn new(registry: &Rc<Registry>) -> Self {
        let world: &WorldRef = registry.world();
        let ctf = Rc::new(Ctf::new(
            Rc::new(CtfRepository::new(world.db().clone())),
            registry.get(),
            registry.get(),
            registry.get(),
        ));

        Self {
            player: Rc::new(PlayerService::new(registry)),
            ctf: ctf.clone(),
            mission_offers: Rc::new(MissionOfferService::new(
                world,
                registry.get(),
                registry.get(),
                registry.get(),
            )),
            game_over: Rc::new(GameOver::new(world, registry)),
        }
    }

    /// Returns the player schema in CTF table creation order.
    #[must_use]
    pub fn schema() -> DomainSchema {
        DomainSchema::new(<Self as Module>::NAME, [storage::schema::ctf::GROUP])
            .with_dependencies(&["core"])
    }
}

impl Module for PlayerModule {
    const NAME: &'static str = "player";

    fn register(registry: &Rc<Registry>) -> Self {
        let module = Self::new(registry);
        let game_over = module.game_over.clone();

        registry.register_async_subscriber(move |strike: &greyhack_world_traces::StrikeEvent| {
            let game_over = game_over.clone();

            Box::pin(async move {
                if strike.strikes >= greyhack_behaviour::MAX_STRIKES {
                    game_over
                        .end_game()
                        .await
                        .map_err(greyhack_world_effects::EventError::new)?;
                    game_over.world.events().publish(GameOverEvent);
                }

                Ok(())
            })
        });
        registry.register_schema(Self::schema());
        registry.register_service(Rc::new(module.clone()));
        registry.register_service(module.player.clone());
        registry.register_service(module.ctf.clone());
        registry.register_service(module.mission_offers.clone());
        registry.register_service(module.game_over.clone());

        module
    }
}
