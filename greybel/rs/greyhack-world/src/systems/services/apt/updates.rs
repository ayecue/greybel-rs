//! Updating package caches and checking for upgrades.

use greyhack_behaviour::{
    APT_CACHE_NAME, APT_FOLDER, FileData, OFFICIAL_OMISSIONS, PermissionSet, ROOT_USER, ServiceId,
    compare_versions, path,
};

use crate::systems::network::connect::ServiceCheck;
use crate::{
    AptFailure, ComputerId, ConnectionFailure, NodeDraft, RepositoryConfig, Result, ServiceFailure,
    World,
};

/// Output line produced by `AptClientIntrinsics.update` for one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AptUpdateLine {
    /// The source at this address is updating.
    Update(String),
    Connection(ConnectionFailure),
    Service(ServiceFailure),
    /// The repository server has no repository configuration file.
    MissingConfig,
    /// The repository configuration does not use Grey Hack's format.
    MalformedConfig,
}

impl World {
    /// Refreshes `aptcache.bin` and returns one output line per source
    /// (`AptClientIntrinsics.update`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn update_apt_packages(
        &mut self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
    ) -> Result<Vec<AptUpdateLine>> {
        let sources = self.apt_sources(computer).await?;
        let official = self.apt_official(computer).await?;
        let mut output = Vec::new();
        let mut servers = serde_json::Map::new();

        for source in &sources {
            output.push(AptUpdateLine::Update(source.address.clone()));

            let Some(address) = self.source_address(source).await? else {
                continue;
            };
            let target = match self
                .connect_computer(computer, &address, source.port)
                .await?
            {
                Ok(target) => target,
                Err(failure) => {
                    output.push(AptUpdateLine::Connection(failure));
                    continue;
                }
            };
            let public = self.public_ip(computer).await?;

            if let Some(failure) = self
                .check_service(&ServiceCheck {
                    target,
                    service: ServiceId::Repository,
                    source: &public,
                    address: &address,
                    port: source.port,
                    firewall: false,
                    terminal_computers,
                })
                .await?
            {
                output.push(AptUpdateLine::Service(failure));
                continue;
            }

            let Some(content) = self
                .content(target, ServiceId::Repository.config_path(), true)
                .await?
            else {
                output.push(AptUpdateLine::MissingConfig);
                continue;
            };
            let Ok(config) = serde_json::from_str::<RepositoryConfig>(&content) else {
                output.push(AptUpdateLine::MalformedConfig);
                continue;
            };

            servers.insert(
                address,
                serde_json::json!({
                    "serverName": source.address,
                    "serverPort": source.port,
                    "pkgList": config.repo_sources,
                }),
            );
        }

        let cache = serde_json::json!({
            "official_server": official,
            "serverList": servers,
        })
        .to_string();
        let data = FileData {
            kind: greyhack_behaviour::FileKind::Binary,
            content: cache,
            ..FileData::default()
        };
        let node = NodeDraft::file(
            APT_CACHE_NAME,
            ROOT_USER,
            PermissionSet::default_for(false),
            data,
        );

        if let Some(folder) = self.folder(computer, APT_FOLDER, true).await? {
            self.add_node(&folder, &node).await?;
        }

        Ok(output)
    }

    /// Validates the cache before `check_upgrade` (`AptClientIntrinsics.check_upgrade`).
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error reports an absent or invalid cache.
    pub async fn validate_apt_cache(&self, computer: ComputerId) -> Result<Result<(), AptFailure>> {
        Ok(self.apt_cache_official(computer).await?.map(|_| ()))
    }

    /// Returns whether library `file_path` has a newer version in its sources.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn check_apt_upgrade(
        &mut self,
        computer: ComputerId,
        file_path: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Option<bool>> {
        if self.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let name = path::file_name(file_path);
        let repositories = self.apt_sources(computer).await?;
        let Some(local) = self.file(computer, file_path, true).await? else {
            return Ok(Some(false));
        };
        let library = self.file_library(computer, file_path, true).await?;

        // The official server answers for the libraries it has, the sources for every other name.
        if matches!(self.apt_cache_official(computer).await?, Ok(true))
            && !OFFICIAL_OMISSIONS.contains(&name)
            && let Some(latest) = self.official_library(name).await?
        {
            let Some(library) = library else {
                return Ok(Some(false));
            };

            return Ok(Some(
                compare_versions(&latest.version, &library.version).is_gt(),
            ));
        }

        for source in &repositories {
            let Some((server, source_path)) = self
                .source_package(computer, source, name, terminal_computers)
                .await?
            else {
                continue;
            };
            let Some(remote) = self.file(server, &source_path, true).await? else {
                continue;
            };

            if let Some(local_library) = library
                && let Some(remote_library) = self.file_library(server, &source_path, true).await?
            {
                return Ok(Some(
                    compare_versions(&remote_library.version, &local_library.version).is_gt(),
                ));
            }

            return Ok(Some(match (local.data(), remote.data()) {
                (Some(local), Some(remote)) => local.content != remote.content,
                _ => false,
            }));
        }

        Ok(Some(false))
    }
}
