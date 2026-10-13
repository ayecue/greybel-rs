//! Installing, searching, and listing packages of the official server and repositories.

use greyhack_behaviour::{
    APT_PACKAGE_SIZE, FileData, FileKind, LibraryKind, OFFICIAL_OMISSIONS, OFFICIAL_SERVER,
    PermissionSet, ROOT_USER, default_apt_install_folder,
};

use crate::{
    AptFailure, ComputerId, Library, NodeDraft, PackageEntry, RepositoryConfig, Result, World,
};

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

impl World {
    /// Installs `package` into `folder`, or its default folder when `folder` is empty
    /// (`AptClientIntrinsics.install`): the official server's library first, when the cache lists it, then the
    /// sources in order.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn install_apt_package(
        &mut self,
        computer: ComputerId,
        user: &str,
        package: &str,
        folder: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<(), AptFailure>> {
        let official = match self.apt_cache_official(computer).await? {
            Ok(official) => official,
            Err(failure) => return Ok(Err(failure)),
        };
        let repositories = self.apt_sources(computer).await?;

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

            if self.computer(computer).await?.is_none() {
                return Ok(Err(AptFailure::ComputerUnavailable));
            }

            let Some(target) = self.folder(computer, folder, true).await? else {
                return Ok(Err(AptFailure::MissingFolder(folder.to_owned())));
            };
            let user = self.user(computer, user).await?;

            if !self.can_modify_contents(&target, user.as_ref()).await? {
                return Ok(Err(AptFailure::NoPermission));
            }

            self.add_node(&target, &node).await?;

            return Ok(Ok(()));
        }

        for source in &repositories {
            let Some((server, source_path)) = self
                .source_package(computer, source, package, terminal_computers)
                .await?
            else {
                continue;
            };
            let Some(node) = self.file(server, &source_path, true).await? else {
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
                .copy_between((server, ROOT_USER), &source_path, (computer, user), folder)
                .await?
                .map(|_| ())
                .map_err(AptFailure::Copy));
        }

        Ok(Err(AptFailure::UnknownPackage(package.to_owned())))
    }

    /// The official server's newest library named `name` (`VersionsControl.lastLibs`), if it has one.
    pub(super) async fn official_library(&self, name: &str) -> Result<Option<Library>> {
        match LibraryKind::of_file(name) {
            Some(kind) => self.latest_library(kind).await,
            None => Ok(None),
        }
    }

    /// Returns source packages whose names contain `term`, or the search failure.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
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
            if let Some((_, config)) = self
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

    /// Returns every package in `repository`, or the listing failure.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn show_apt_repository(
        &self,
        computer: ComputerId,
        repository: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<Vec<PackageEntry>, AptFailure>> {
        if let Err(failure) = self.apt_cache_official(computer).await? {
            return Ok(Err(failure));
        }

        // `AptClientIntrinsics.show` answers `official_server` whatever the cache's `official_server` says.
        if repository == OFFICIAL_SERVER {
            return Ok(Ok(official_packages().collect()));
        }

        for source in &self.apt_sources(computer).await? {
            if source.address != repository {
                continue;
            }

            let Some((_, config)) = self
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
