//! Worlds generated in process.

use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions};
use futures_util::future::LocalBoxFuture;
use greybel_interpreter::Clock;
use greybel_runtime::{LaunchWorld, WorldFactory};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::ServerConnection;
use scheduler::Scheduler;

/// In-process world server with the default player's world.
pub struct LocalWorld {
    server: WorldServer,
}

impl LocalWorld {
    /// Generates the world of `seed`; `clock` drives its time.
    ///
    /// # Panics
    /// Panics when the world can't be generated or its server can't restore payments.
    pub async fn generate(seed: &str, clock: Rc<dyn Clock>) -> Self {
        let world = World::from_seed(seed, &PlayerOptions::default())
            .await
            .expect("the world generates");
        let server = WorldServer::new(world, clock)
            .await
            .expect("the server restores payments");

        Self { server }
    }

    #[must_use]
    pub fn connect<S: Scheduler>(&self, scheduler: S) -> ServerConnection {
        ServerConnection::new(Channel::new(
            &self.server.connect(scheduler),
            ChannelOptions::default(),
        ))
    }

    #[must_use]
    pub fn connect_on_tokio(&self) -> ServerConnection {
        ServerConnection::new(Channel::new(
            &self.server.connect_on_tokio(),
            ChannelOptions::default(),
        ))
    }
}

/// Generates a [`LocalWorld`] for each launch.
pub struct LocalWorldFactory;

impl<S: Scheduler> WorldFactory<S> for LocalWorldFactory {
    fn create(
        &self,
        seed: &str,
        clock: Rc<dyn Clock>,
        scheduler: S,
    ) -> LocalBoxFuture<'static, LaunchWorld> {
        let seed = seed.to_owned();

        Box::pin(async move {
            let world = LocalWorld::generate(&seed, clock).await;

            LaunchWorld {
                connection: world.connect(scheduler),
                guard: Some(Box::new(world)),
            }
        })
    }
}
