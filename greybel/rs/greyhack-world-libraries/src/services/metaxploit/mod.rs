//! Metaxploit: scanning libraries, opening sessions, and reverse shells.

pub(crate) mod net_use;
pub(crate) mod rshell;
pub(crate) mod scan;

use greyhack_world_intrusion::{LibraryObjectAccess, ObjectAccessFuture};

use crate::objects::{NetSessionRef, ObjectTableExt as _};
use crate::{ComputerId, Result, ReverseShell, RouterId};

impl LibraryObjectAccess for Metaxploit {
    fn metaxploit_access(
        &self,
        objects: &greyhack_world_router::ObjectTableHandle<ComputerId>,
        object: greyhack_world_router::protocol::ObjectId,
    ) -> greyhack_world_router::RouteResult<
        greyhack_world_core::Access,
        greyhack_world_core::WorldError,
    > {
        objects.metaxploit(object).map(|meta| meta.access)
    }

    fn flood_session<'a>(
        &'a self,
        objects: &'a greyhack_world_router::ObjectTableHandle<ComputerId>,
        object: greyhack_world_router::protocol::ObjectId,
    ) -> ObjectAccessFuture<'a, Option<(ComputerId, ComputerId)>> {
        Box::pin(async move {
            let session = objects.net_session(object)?;
            let Some(computer) = self.session_computer(&session).await? else {
                return Ok(None);
            };

            Ok(Some((computer, session.origin())))
        })
    }
}

/// A world service with only its declared dependencies.
pub struct Metaxploit {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) reverse_shells: std::rc::Rc<crate::storage::ReverseShellRepository>,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) generator: greyhack_world_core::Dep<crate::Generator>,
    pub(crate) network_generation:
        greyhack_world_core::Dep<dyn greyhack_world_core::NetworkGeneration>,
}

impl Metaxploit {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        reverse_shells: std::rc::Rc<crate::storage::ReverseShellRepository>,
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_core::Dep<greyhack_world_core::FileSystems>,
        networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
        generator: greyhack_world_core::Dep<crate::Generator>,
        network_generation: greyhack_world_core::Dep<dyn greyhack_world_core::NetworkGeneration>,
    ) -> Self {
        Self {
            world: world.clone(),
            reverse_shells,
            computers,
            files,
            networks,
            generator,
            network_generation,
        }
    }
}

impl Metaxploit {
    /// Returns the service or router computer of `session` if it still exists.
    pub(crate) async fn session_computer(
        &self,
        session: &NetSessionRef,
    ) -> Result<Option<ComputerId>> {
        let id = match session {
            NetSessionRef::Service { file, .. } => file.access.computer,
            NetSessionRef::Router(kernel) => kernel.router.computer(),
        };

        Ok(self.networks.topology.computer(id).await?.map(|_| id))
    }

    /// Returns the reverse shells registered on `router`.
    pub async fn reverse_shells(&self, router: RouterId) -> Result<Vec<ReverseShell>> {
        self.reverse_shells.reverse_shells(router).await
    }
}

impl Metaxploit {
    /// Registers a reverse shell on `router`.
    pub async fn insert_reverse_shell(&self, router: RouterId, shell: &ReverseShell) -> Result<()> {
        self.reverse_shells
            .insert_reverse_shell(router, shell)
            .await
    }
}
