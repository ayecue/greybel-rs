//! Library updates a network reports (`Persona.UpdateReportedNetworks`, `NetworkLan.ReportedLibUpdate`,
//! `FileSystem.ApplyReportedLibUpdates`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use greyhack_behaviour::{
    FileKind, LIBRARY_FOLDER, LibraryKind, ROUTER_NODE_LABEL, Rng, TOP_LEVEL_DOMAINS,
    compare_versions, library_path,
};
use greyhack_world_core::{ComputerId, LibraryId, NetworkId, Result};
use mono_random::Random;

use crate::{ComputerDraft, Library};

type ChoiceSet = Vec<(LibraryKind, Library)>;

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
        self.libraries
            .set_network_library_choice(router.network, newest.0, newest.1)
            .await?;
        self.apply_reported_library_updates(router.network).await?;

        Ok(Some(Ok(())))
    }

    pub async fn register_library(&self, library: &Library) -> Result<()> {
        self.libraries.library_id(library).await?;
        self.libraries
            .files
            .library_row(library.kind, &library.version)
            .await?;

        Ok(())
    }

    pub async fn register_saved_library(&self, library: &Library) -> Result<()> {
        let id = self.libraries.library_id(library).await?;

        self.libraries
            .files
            .library_row(library.kind, &library.version)
            .await?;

        self.library_versions.save_library(library.kind, id).await
    }

    pub async fn start_with_reported_libraries(
        &self,
        network: NetworkId,
        draft: &mut ComputerDraft,
    ) -> Result<()> {
        self.apply_network_choices(network, draft).await?;

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
    async fn choice_records(&self, network: NetworkId) -> Result<Rc<ChoiceSet>> {
        if let Some(records) = self.choice_records.borrow().get(&network).cloned() {
            return Ok(records);
        }

        let mut records = Vec::new();

        for (kind, choice) in self.libraries.network_library_choices(network).await? {
            if let Some(library) = self.libraries.library(choice).await? {
                records.push((kind, library));
            }
        }

        let records = Rc::new(records);

        self.choice_records
            .borrow_mut()
            .insert(network, records.clone());

        Ok(records)
    }

    /// Returns the router-node's selected version for each library kind.
    pub async fn network_library_versions(
        &self,
        network: NetworkId,
    ) -> Result<Vec<(LibraryKind, String)>> {
        let mut versions = Vec::new();

        for (kind, library) in self.choice_records(network).await?.iter() {
            versions.push((*kind, library.version.clone()));
        }

        Ok(versions)
    }

    /// Reproduces the router-node draw: its top-level-domain draw precedes one index per saved library kind.
    pub async fn ensure_network_library_choices(
        &self,
        network: NetworkId,
        public_ip: &str,
    ) -> Result<()> {
        if !self
            .libraries
            .network_library_choices(network)
            .await?
            .is_empty()
        {
            return Ok(());
        }

        let mut random = Random::new(
            self.world
                .world_seed()
                .derive_int(ROUTER_NODE_LABEL, &[public_ip]),
        );

        random.next_below(TOP_LEVEL_DOMAINS.len());

        for kind in LibraryKind::ALL.iter().copied() {
            let versions = self.library_versions.saved_libraries(kind).await?;

            if versions.is_empty() {
                continue;
            }

            let library = versions[random.next_below(versions.len())];

            self.libraries
                .set_network_library_choice(network, kind, library)
                .await?;
        }

        self.choice_records.borrow_mut().remove(&network);

        Ok(())
    }

    /// Applies an explicitly mirrored router-node choice once the saved contents have been registered.
    pub async fn set_network_library_choices(
        &self,
        network: NetworkId,
        choices: &[(LibraryKind, String)],
    ) -> Result<()> {
        for (kind, version) in choices {
            if let Some(library) = self.files.library_of(*kind, version).await? {
                self.libraries
                    .set_network_library_choice(network, *kind, library)
                    .await?;
            }
        }

        self.choice_records.borrow_mut().remove(&network);

        Ok(())
    }

    /// Uses a network's saved version for every library on a generated computer.
    pub async fn apply_network_choices(
        &self,
        network: NetworkId,
        draft: &mut ComputerDraft,
    ) -> Result<()> {
        for (kind, library) in self.choice_records(network).await?.iter() {
            let kind = *kind;
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

    /// The player computer uses the random saved-version overload rather than its network's choice.
    pub async fn apply_random_saved_libraries(
        &self,
        draft: &mut ComputerDraft,
        rng: &mut Rng,
    ) -> Result<()> {
        for kind in LibraryKind::ALL.iter().copied() {
            let versions = self.library_versions.saved_libraries(kind).await?;

            if versions.is_empty() {
                continue;
            }

            let Some(library) = self
                .libraries
                .library(versions[rng.index(versions.len())])
                .await?
            else {
                continue;
            };
            let fs = &mut draft.filesystem;
            let Some(folder) = fs.folder(LIBRARY_FOLDER, false) else {
                continue;
            };

            for file in fs.files(folder).to_vec() {
                let node = fs.node_mut(file);

                if node
                    .data()
                    .is_some_and(|data| data.kind == FileKind::Library)
                    && LibraryKind::of_file(&node.name) == Some(kind)
                {
                    node.library = Some(Box::new(library.clone()));
                }
            }
        }

        Ok(())
    }

    /// Refreshes generated computers after a mirrored history has been registered.
    pub async fn apply_network_choices_to_computers(&self, network: NetworkId) -> Result<()> {
        for computer in self.computers.computers_in(network).await? {
            if computer.id == self.world.player() {
                continue;
            }

            for (kind, choice) in self.libraries.network_library_choices(network).await? {
                if self.libraries.library(choice).await?.is_none() {
                    continue;
                }

                let path = library_path(kind.file_name());
                let Some(node) = self.files.lookup.file(computer.id, &path, false).await? else {
                    continue;
                };

                // Mirrored fixtures can install a library with authored weaknesses or malformed content. A saved choice
                // only fills an unwritten node; it must not replace that explicit content or give it a valid identity.
                if node.data().is_some_and(|data| !data.content.is_empty()) {
                    continue;
                }

                if self.files.node_library(node.id).await?.is_some() {
                    continue;
                }

                if node
                    .data()
                    .is_some_and(|data| data.kind == FileKind::Library)
                {
                    self.files.set_node_library(node.id, Some(choice)).await?;
                }
            }
        }

        Ok(())
    }

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
    pub(crate) library_versions: std::rc::Rc<crate::storage::LibraryVersionRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
    pub(crate) generator: greyhack_world_modules::Dep<crate::Generator>,
    /// Full selected libraries, read once per network instead of once per generated computer.
    choice_records: RefCell<HashMap<NetworkId, Rc<ChoiceSet>>>,
}

impl UpdateService {
    #[expect(
        clippy::too_many_arguments,
        reason = "The service receives its independent repositories and domain dependencies from the module root"
    )]
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        libraries: std::rc::Rc<crate::storage::LibraryRepository>,
        library_versions: std::rc::Rc<crate::storage::LibraryVersionRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        guilds: greyhack_world_modules::Dep<greyhack_world_guilds::GuildService>,
        generator: greyhack_world_modules::Dep<crate::Generator>,
    ) -> Self {
        Self {
            world: world.clone(),
            libraries,
            library_versions,
            computers,
            files,
            networks,
            guilds,
            generator,
            choice_records: RefCell::new(HashMap::new()),
        }
    }
}
