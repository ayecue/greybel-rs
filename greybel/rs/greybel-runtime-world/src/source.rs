use std::rc::Rc;

use greybel_interpreter::Clock;
use greybel_runtime::protocol::LaunchFailure;
use greybel_runtime::{LaunchWorld, SharedWorld, SharedWorldFailure, connect_over_tcp};
use greyscript_intrinsics::ServerConnection;

use crate::LocalWorld;

/// Where a host without an editor finds the world of a simulated launch.
#[derive(Clone, Debug)]
pub enum WorldSource {
    Address(String),
    /// World generated in process for each launch.
    Own,
    /// World server that the runtimes of this machine share. A launch gets a world of its own when the server runs
    /// another seed or can't be used.
    Machine(SharedWorld),
}

impl WorldSource {
    /// World of a launch of `seed` whose in-process world runs on `clock`. `report` receives why the shared server
    /// wasn't used.
    ///
    /// # Errors
    ///
    /// Returns [`LaunchFailure::Unreachable`] when the world server at an address can't be reached.
    pub async fn launch_world(
        &self,
        seed: &str,
        clock: Rc<dyn Clock>,
        report: impl FnOnce(&SharedWorldFailure),
    ) -> Result<LaunchWorld, LaunchFailure> {
        match self {
            Self::Address(address) => {
                connect_over_tcp(address)
                    .await
                    .map(Self::guard)
                    .map_err(|error| LaunchFailure::Unreachable {
                        message: format!("the world server at {address} is unreachable: {error}"),
                    })
            }
            Self::Own => Ok(Self::own(seed, clock).await),
            Self::Machine(world) => match world.connect(seed).await {
                Ok(connection) => Ok(Self::guard(connection)),
                Err(failure) => {
                    report(&failure);

                    Ok(Self::own(seed, clock).await)
                }
            },
        }
    }

    /// World of `connection`, which closes when the launch ends.
    fn guard(connection: ServerConnection) -> LaunchWorld {
        LaunchWorld {
            connection: connection.clone(),
            guard: Some(Box::new(ConnectionGuard(connection))),
        }
    }

    async fn own(seed: &str, clock: Rc<dyn Clock>) -> LaunchWorld {
        let world = LocalWorld::generate(seed, clock).await;

        LaunchWorld {
            connection: world.connect_on_tokio(),
            guard: Some(Box::new(world)),
        }
    }
}

struct ConnectionGuard(ServerConnection);

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.0.close();
    }
}
