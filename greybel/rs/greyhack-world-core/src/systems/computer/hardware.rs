//! Computer hardware, ports, and library versions.

use std::rc::Rc;

use crate::storage::*;
use crate::systems::*;
use crate::*;

impl Hardware {
    /// Inserts a complete computer draft, composing the core repositories in write order.
    pub async fn insert_computer_at_with<L, S>(
        &self,
        draft: &ComputerDraft<L, S>,
        network: NetworkId,
        place: Option<&LanPlace>,
        library_id: impl AsyncFn(&L) -> Result<crate::LibraryId>,
    ) -> Result<ComputerId> {
        let id = self
            .computer_rows
            .insert_computer_row(draft, network, place)
            .await?;

        self.hardware_rows
            .insert_hardware(id, &draft.hardware)
            .await?;

        for user in &draft.users {
            self.user_rows.insert_user(id, user).await?;
        }

        for process in &draft.processes {
            self.computer_rows.insert_process(id, process).await?;
        }

        self.node_rows
            .insert_filesystem_with(id, &draft.filesystem, library_id)
            .await?;

        for port in &draft.ports {
            let library = self
                .library_rows
                .library_row(port.service.library_kind(), &port.version)
                .await?;

            self.computer_rows.insert_port(id, port, library).await?;
        }

        for network in &draft.saved_networks {
            let password_hash = self.users.register_password(&network.password).await?;

            self.computer_rows
                .add_saved_network(id, network, &password_hash)
                .await?;
        }

        for user in &draft.active_users {
            self.computer_rows.insert_session(id, user).await?;
        }

        Ok(id)
    }

    /// Returns every part installed in `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn hardware(&self, computer: ComputerId) -> Result<crate::model::Hardware> {
        self.hardware_rows.hardware(computer).await
    }

    /// Returns `computer` port `number`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port(&self, computer: ComputerId, number: u16) -> Result<Option<Port>> {
        self.computer_rows.port(computer, number).await
    }

    /// Changes library file `path` on `computer` to `version`, preserving kind and weaknesses, and
    /// returns whether it was a library. Service files and versions correspond to
    /// `LanPcInfo.libVersions` (`RedGlobal.ShouldRestore`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_library_version(
        &self,
        computer: ComputerId,
        path: &str,
        version: &str,
    ) -> Result<bool> {
        let hooks = self.libraries.clone();

        hooks
            .set_library_version(&self.world, computer, path, version)
            .await
    }

    /// Changes `computer` port `number` to its service library at `version` and returns whether the
    /// port exists. Grey Hack associates each service with a versioned client library
    /// (`RedGlobal.ShouldRestore`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_port_library_version(
        &self,
        computer: ComputerId,
        number: u16,
        version: &str,
    ) -> Result<bool> {
        let Some(port) = self.port(computer, number).await? else {
            return Ok(false);
        };
        let library = self
            .library_rows
            .library_row(port.service.library_kind(), version)
            .await?;

        self.computer_rows
            .set_port_library(computer, number, library)
            .await
    }
}

/// Computer hardware and library-port rules.
pub struct Hardware {
    pub(crate) world: WorldRef,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) hardware_rows: Rc<HardwareRepository>,
    pub(crate) node_rows: Rc<NodeRepository>,
    pub(crate) user_rows: Rc<UserRepository>,
    pub(crate) library_rows: Rc<LibraryIdentityRepository>,
    pub(crate) users: Dep<UserService>,
    pub libraries: Dep<dyn crate::LibraryVersions>,
}

impl Hardware {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        hardware_rows: Rc<HardwareRepository>,
        node_rows: Rc<NodeRepository>,
        user_rows: Rc<UserRepository>,
        library_rows: Rc<LibraryIdentityRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            libraries: registry.get(),
            computer_rows,
            hardware_rows,
            node_rows,
            user_rows,
            library_rows,
            users: registry.get(),
        }
    }
}
