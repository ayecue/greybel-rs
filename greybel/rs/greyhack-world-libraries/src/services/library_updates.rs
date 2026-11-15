//! Library updates a network reports (`Persona.UpdateReportedNetworks`, `NetworkLan.ReportedLibUpdate`,
//! `FileSystem.ApplyReportedLibUpdates`).

use greyhack_behaviour::{FileKind, LIBRARY_FOLDER, LibraryKind, compare_versions, library_path};
use greyhack_world_core::{ComputerId, LibraryId, NetworkId, Result};

use crate::{ComputerDraft, Library};

/// Why a library update was not reported (`Persona.UpdateReportedNetworks`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryReportFailure {
    /// The world has no library of that name (`VersionsControl.lastLibs`).
    UnknownLibrary,
    /// The guild owning the company turned Admin Restore off for it (`Guilds.IsAdminRestoreDisabledForCompany`).
    AdminRestoreOff,
}

/// Methods of [`World`] for exploits library updates.
impl UpdateService {
    pub async fn report_library_update(
        &self,
        public_ip: &str,
        library: &str,
    ) -> Result<Option<Result<(), LibraryReportFailure>>> {
        let Some(router) = self.networks.router_at(public_ip).await? else {
            return Ok(None);
        };

        // The guild's switch is checked before the library is looked up (`Persona.UpdateReportedNetworks`).
        if self.guilds.is_admin_restore_disabled(public_ip).await? {
            return Ok(Some(Err(LibraryReportFailure::AdminRestoreOff)));
        }

        let Some(newest) = self.newest_library_of_file(library).await? else {
            return Ok(Some(Err(LibraryReportFailure::UnknownLibrary)));
        };

        self.libraries
            .set_network_library_update(router.network, newest.0, newest.1)
            .await?;
        self.apply_reported_library_updates(router.network).await?;

        Ok(Some(Ok(())))
    }

    pub async fn register_library(&self, library: &Library) -> Result<()> {
        self.libraries.library_id(library).await?;

        Ok(())
    }

    pub async fn start_with_reported_libraries(
        &self,
        network: NetworkId,
        draft: &mut ComputerDraft,
    ) -> Result<()> {
        for (kind, update) in self.libraries.network_library_updates(network).await? {
            let Some(library) = self.libraries.library(update).await? else {
                continue;
            };
            let fs = &mut draft.filesystem;

            if let Some(folder) = fs.folder(LIBRARY_FOLDER, false) {
                for file in fs.files(folder).to_vec() {
                    let node = fs.node_mut(file);
                    let is_library = node
                        .data()
                        .is_some_and(|data| data.kind == FileKind::Library);

                    if is_library && LibraryKind::of_file(&node.name) == Some(kind) {
                        node.library = Some(Box::new(library.clone()));
                    }
                }
            }

            for port in &mut draft.ports {
                if port.service.library_kind() == kind {
                    port.version.clone_from(&library.version);
                }
            }
        }

        Ok(())
    }
}

impl UpdateService {
    pub async fn newest_library_of_file(
        &self,
        name: &str,
    ) -> Result<Option<(LibraryKind, LibraryId)>> {
        let Some(kind) = LibraryKind::of_file(name) else {
            return Ok(None);
        };

        Ok(self
            .files
            .latest_library(kind)
            .await?
            .map(|library| (kind, library)))
    }

    pub async fn apply_reported_library_updates(&self, network: NetworkId) -> Result<()> {
        let updates = self.libraries.network_library_updates(network).await?;

        for computer in self.computers.computers_in(network).await? {
            if computer.id == self.world.player() {
                continue;
            }

            for (kind, update) in &updates {
                self.apply_reported_library_update(computer.id, *kind, *update)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn apply_reported_library_update(
        &self,
        computer: ComputerId,
        kind: LibraryKind,
        update: LibraryId,
    ) -> Result<()> {
        let path = library_path(kind.file_name());
        let Some(node) = self.files.lookup.file(computer, &path, false).await? else {
            return Ok(());
        };

        if node
            .data()
            .is_none_or(|data| data.kind != FileKind::Library)
        {
            return Ok(());
        }

        let (Some(current), Some(newer)) = (
            self.generator.full_library_of(&node).await?,
            self.libraries.library(update).await?,
        ) else {
            return Ok(());
        };

        if compare_versions(&current.version, &newer.version).is_lt() {
            self.files.set_node_library(node.id, Some(update)).await?;
        }

        Ok(())
    }
}

/// A world service with only its declared dependencies.
pub struct UpdateService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) libraries: std::rc::Rc<crate::storage::LibraryRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
    pub(crate) generator: greyhack_world_modules::Dep<crate::Generator>,
}

impl UpdateService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        libraries: std::rc::Rc<crate::storage::LibraryRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
        generator: greyhack_world_modules::Dep<crate::Generator>,
    ) -> Self {
        Self {
            world: world.clone(),
            libraries,
            computers,
            files,
            networks,
            guilds,
            generator,
        }
    }
}
