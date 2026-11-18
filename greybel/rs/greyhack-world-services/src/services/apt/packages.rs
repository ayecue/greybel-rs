//! Installing, searching, and listing packages of the official server and repositories.

use greyhack_behaviour::{
    APT_PACKAGE_SIZE, FileData, FileKind, LibraryKind, OFFICIAL_OMISSIONS, OFFICIAL_SERVER,
    PermissionSet, ROOT_USER, default_apt_install_folder, file_size_mb, path,
};
use greyhack_world_core::{ComputerId, LibraryId, NodeDraft, Result, WorldError};

use super::json_deserializes_to_null;
use super::updates::AptCache;
use crate::services::Apt;
use crate::{AptFailure, PackageEntry, RepositoryConfig};

/// Official-server packages: all libraries except explicit omissions.
pub(super) fn official_packages() -> impl Iterator<Item = PackageEntry> {
    LibraryKind::ALL
        .iter()
        .map(|kind| kind.file_name())
        .filter(|name| !OFFICIAL_OMISSIONS.contains(&name.as_str()))
        .map(|name| PackageEntry {
            name,
            description: None,
        })
}

/// Packages a repository server lists in its configuration, in file order.
pub(super) fn repository_packages(config: &RepositoryConfig) -> impl Iterator<Item = PackageEntry> {
    config
        .repo_sources
        .iter()
        .map(|(name, description)| PackageEntry {
            name: name.clone(),
            description: Some(description.clone()),
        })
}

/// Methods of [`World`] for services apt packages.
impl Apt {
    #[expect(
        clippy::too_many_lines,
        reason = "Apt install preserves the game's validation and repository error order"
    )]
    pub async fn install_apt_package(
        &self,
        computer: ComputerId,
        user: &str,
        package: &str,
        folder: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<(), AptFailure>> {
        if !self
            .networks
            .reach
            .is_network_active(computer, terminal_computers)
            .await?
        {
            return Ok(Err(AptFailure::NoInternet));
        }

        let content = match self.apt_cache_content(computer).await? {
            Ok(content) => content,
            Err(failure) => return Ok(Err(failure)),
        };

        if json_deserializes_to_null(&content) {
            return Err(WorldError::GameCrash);
        }

        let Ok(cache) = serde_json::from_str::<AptCache>(&content) else {
            return Ok(Err(AptFailure::MalformedCache));
        };
        let official = cache.official_server;

        if official
            && !OFFICIAL_OMISSIONS.contains(&package)
            && let Some(library) = self.official_library(package).await?
        {
            let folder = if folder.is_empty() {
                default_apt_install_folder(true)
            } else {
                folder
            };
            let mut node = NodeDraft::file(
                package,
                user,
                PermissionSet::default_for(false),
                FileData::library(APT_PACKAGE_SIZE),
            );

            node.library = Some(Box::new(library));

            if self.networks.topology.computer(computer).await?.is_none() {
                return Ok(Err(AptFailure::ComputerUnavailable));
            }

            let Some(target) = self.files.lookup.folder(computer, folder, true).await? else {
                return Ok(Err(AptFailure::MissingFolder(folder.to_owned())));
            };

            if file_size_mb(APT_PACKAGE_SIZE)
                > self.files.sizes.available_storage_mb(computer).await?
            {
                return Ok(Err(AptFailure::NoSpace));
            }

            let user = self.computers.users.user(computer, user).await?;

            if !self
                .files
                .permissions
                .can_modify_contents(&target, user.as_ref())
                .await?
            {
                return Ok(Err(AptFailure::NoPermission));
            }

            self.files
                .changes
                .add_node_with(&target, &node, async |id| Ok(*id))
                .await?;

            return Ok(Ok(()));
        }

        for (address, server_entry) in &cache.server_list {
            if !server_entry.package_list.contains_key(package) {
                continue;
            }

            let source = greyhack_behaviour::AptSource {
                address: address.clone(),
                port: server_entry.server_port,
            };
            let (server, source_path) = match self
                .source_package(computer, &source, package, terminal_computers)
                .await?
            {
                Ok(package) => package,
                Err(AptFailure::UnknownRepository(_)) => continue,
                Err(failure) => return Ok(Err(failure)),
            };

            if self
                .files
                .lookup
                .folder(server, path::parent(&source_path), true)
                .await?
                .is_none()
            {
                return Ok(Err(AptFailure::MissingRepositoryFolder));
            }

            let Some(node) = self.files.lookup.file(server, &source_path, true).await? else {
                return Ok(Err(AptFailure::MissingPackageFile(package.to_owned())));
            };
            let folder = if folder.is_empty() {
                default_apt_install_folder(
                    node.data()
                        .is_some_and(|data| data.kind == FileKind::Library),
                )
            } else {
                folder
            };

            return Ok(self
                .files
                .changes
                .copy_between((server, ROOT_USER), &source_path, (computer, user), folder)
                .await?
                .map(|_| ())
                .map_err(AptFailure::Copy));
        }

        Ok(Err(AptFailure::UnknownPackage(package.to_owned())))
    }

    pub async fn official_library(&self, name: &str) -> Result<Option<LibraryId>> {
        match LibraryKind::of_file(name) {
            Some(kind) => self.files.latest_library(kind).await,
            None => Ok(None),
        }
    }

    pub async fn search_apt_packages(
        &self,
        computer: ComputerId,
        term: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<Vec<PackageEntry>, AptFailure>> {
        let official = match self.apt_cache_official(computer).await? {
            Ok(official) => official,
            Err(failure) => return Ok(Err(failure)),
        };
        let mut packages = Vec::new();

        // `AptClientIntrinsics.search` lists the official server's libraries, then every source's packages.
        if official {
            packages.extend(official_packages().filter(|package| package.name.contains(term)));
        }

        for source in &self.apt_sources(computer).await? {
            if let Ok((_, config)) = self
                .remote_repository(computer, source, terminal_computers)
                .await?
            {
                packages.extend(
                    repository_packages(&config).filter(|package| package.name.contains(term)),
                );
            }
        }

        Ok(if packages.is_empty() {
            Err(AptFailure::NoMatch(term.to_owned()))
        } else {
            Ok(packages)
        })
    }

    pub async fn show_apt_repository(
        &self,
        computer: ComputerId,
        repository: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<Vec<PackageEntry>, AptFailure>> {
        if let Err(failure) = self.apt_cache_content(computer).await? {
            return Ok(Err(failure));
        }

        // `AptClientIntrinsics.show` answers `official_server` before it deserializes the cache.
        if repository == OFFICIAL_SERVER {
            return Ok(Ok(official_packages().collect()));
        }

        if let Err(failure) = self.apt_cache_official(computer).await? {
            return Ok(Err(failure));
        }

        for source in &self.apt_sources(computer).await? {
            if source.address != repository {
                continue;
            }

            let Ok((_, config)) = self
                .remote_repository(computer, source, terminal_computers)
                .await?
            else {
                continue;
            };

            return Ok(Ok(repository_packages(&config).collect()));
        }

        Ok(Err(AptFailure::UnknownRepository(repository.to_owned())))
    }
}
