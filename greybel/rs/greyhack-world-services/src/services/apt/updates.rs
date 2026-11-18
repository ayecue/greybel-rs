//! Updating package caches and checking for upgrades.

use greyhack_behaviour::{
    APT_CACHE_NAME, APT_FOLDER, APT_SOURCES_PATH, DOWNLOADS_PATH, FileData, FileKind,
    OFFICIAL_OMISSIONS, PermissionSet, ROOT_USER, ServiceId, compare_versions, format_apt_sources,
    path,
};
use greyhack_world_core::systems::network::connect::ServiceCheck;
use greyhack_world_core::{
    ComputerId, ConnectionFailure, NodeDraft, Result, ServiceFailure, WorldError,
};
use indexmap::IndexMap;
use serde::Deserialize;

use super::json_deserializes_to_null;
use crate::services::Apt;
use crate::{AptFailure, AptUpgradeReport, RepositoryConfig};

/// Output line produced by `AptClientIntrinsics.update` for one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AptUpdateLine {
    /// `sources.txt` did not exist and was created with the default configuration.
    MissingSources,
    /// `sources.txt` contains malformed JSON.
    MalformedSources,
    /// No router exists for a configured domain.
    UnknownRepository(String),
    /// The source at this address is updating.
    Update(String),
    Connection(ConnectionFailure),
    Service(ServiceFailure),
    /// The repository server has no repository configuration file.
    MissingConfig,
    /// The repository configuration is a binary file.
    InvalidConfig,
    /// The repository configuration does not use Grey Hack's format.
    MalformedConfig,
}

/// Methods of [`World`] for services apt updates.
impl Apt {
    #[expect(
        clippy::too_many_lines,
        reason = "Apt update follows the game's source-file and per-repository error order"
    )]
    pub async fn update_apt_packages(
        &self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
    ) -> Result<Vec<AptUpdateLine>> {
        let source_file = self
            .files
            .lookup
            .file(computer, APT_SOURCES_PATH, true)
            .await?;

        if source_file.is_none() {
            if let Some(folder) = self.files.lookup.folder(computer, APT_FOLDER, true).await? {
                let sources = NodeDraft::file(
                    "sources.txt",
                    ROOT_USER,
                    PermissionSet::default_for(false),
                    FileData::text(&format_apt_sources(&[], true)),
                );

                self.files.changes.add_node(&folder, &sources).await?;
            }

            return Ok(vec![AptUpdateLine::MissingSources]);
        }

        if source_file.as_ref().is_some_and(|node| {
            node.data()
                .is_some_and(|data| json_deserializes_to_null(&data.content))
        }) {
            return Err(WorldError::GameCrash);
        }

        if source_file.as_ref().is_some_and(|node| {
            node.data().is_some_and(|data| {
                serde_json::from_str::<serde_json::Value>(&data.content).is_err()
            })
        }) {
            return Ok(vec![AptUpdateLine::MalformedSources]);
        }

        let sources = self.apt_sources(computer).await?;
        let official = self.apt_official(computer).await?;
        let mut output = Vec::new();
        let mut servers = serde_json::Map::new();

        for source in &sources {
            let Some(address) = self.source_address(source).await? else {
                output.push(AptUpdateLine::UnknownRepository(source.address.clone()));
                continue;
            };

            output.push(AptUpdateLine::Update(source.address.clone()));

            let target = match self
                .networks
                .connect
                .connect_computer(computer, &address, source.port)
                .await?
            {
                Ok(target) => target,
                Err(failure) => {
                    output.push(AptUpdateLine::Connection(failure));
                    continue;
                }
            };
            let public = self.networks.topology.public_ip(computer).await?;

            if let Some(failure) = self
                .networks
                .connect
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

            let Some(node) = self
                .files
                .lookup
                .file(target, ServiceId::Repository.config_path(), true)
                .await?
            else {
                output.push(AptUpdateLine::MissingConfig);
                continue;
            };

            let Some(data) = node.data() else {
                output.push(AptUpdateLine::InvalidConfig);
                continue;
            };

            if data.kind == greyhack_behaviour::FileKind::Binary {
                output.push(AptUpdateLine::InvalidConfig);
                continue;
            }

            if json_deserializes_to_null(&data.content) {
                return Err(WorldError::GameCrash);
            }

            let Ok(config) = serde_json::from_str::<RepositoryConfig>(&data.content) else {
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

        if let Some(folder) = self.files.lookup.folder(computer, APT_FOLDER, true).await? {
            self.files.changes.add_node(&folder, &node).await?;
        }

        Ok(output)
    }

    pub async fn validate_apt_cache(&self, computer: ComputerId) -> Result<Result<(), AptFailure>> {
        Ok(self.apt_cache_official(computer).await?.map(|_| ()))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Repository connection, config, and package-version checks follow C# order"
    )]
    async fn check_repository_upgrade(
        &self,
        computer: ComputerId,
        file_path: &str,
        name: &str,
        address: &str,
        server: &CachedServer,
        terminal_computers: &[ComputerId],
    ) -> Result<RepositoryUpgrade> {
        let target = match self
            .networks
            .connect
            .connect_computer(computer, address, server.server_port)
            .await?
        {
            Ok(target) => target,
            Err(failure) => return Ok(RepositoryUpgrade::Failure(AptFailure::Connection(failure))),
        };
        let public = self.networks.topology.public_ip(computer).await?;

        if let Some(failure) = self
            .networks
            .connect
            .check_service(&ServiceCheck {
                target,
                service: ServiceId::Repository,
                source: &public,
                address,
                port: server.server_port,
                firewall: false,
                terminal_computers,
            })
            .await?
        {
            return Ok(RepositoryUpgrade::Failure(AptFailure::Service(failure)));
        }

        let Some(node) = self
            .files
            .lookup
            .file(target, ServiceId::Repository.config_path(), true)
            .await?
        else {
            return Ok(RepositoryUpgrade::Failure(AptFailure::MissingConfig));
        };
        let Some(data) = node.data() else {
            return Ok(RepositoryUpgrade::Failure(AptFailure::MissingConfig));
        };

        self.installs
            .update_hackshop_libs(target, DOWNLOADS_PATH)
            .await?;

        if json_deserializes_to_null(&data.content) {
            return Err(WorldError::GameCrash);
        }

        let Ok(config) = serde_json::from_str::<RepositoryConfig>(&data.content) else {
            return Ok(RepositoryUpgrade::MalformedConfig);
        };

        if self
            .files
            .lookup
            .folder(target, &config.path_repo, true)
            .await?
            .is_none()
        {
            return Ok(RepositoryUpgrade::Failure(
                AptFailure::MissingRepositoryFolder,
            ));
        }

        let remote_path = path::join(&config.path_repo, name);

        if self
            .files
            .lookup
            .file(target, &remote_path, true)
            .await?
            .is_none()
        {
            return Ok(RepositoryUpgrade::Failure(AptFailure::MissingPackageFile(
                name.to_owned(),
            )));
        }

        let Some(local) = self.files.lookup.file(computer, file_path, true).await? else {
            return Ok(RepositoryUpgrade::Failure(AptFailure::MissingLocalFile(
                name.to_owned(),
            )));
        };
        let remote = self
            .files
            .lookup
            .file(target, &remote_path, true)
            .await?
            .expect("the file was found");

        if local
            .data()
            .is_some_and(|data| data.kind == FileKind::Library)
            && remote
                .data()
                .is_some_and(|data| data.kind == FileKind::Library)
        {
            let local_version = self
                .files
                .libraries
                .file_library(computer, file_path, true)
                .await?
                .map(|library| library.version)
                .or_else(|| {
                    local
                        .data()
                        .and_then(|data| serialized_library_version(&data.content))
                });
            let remote_version = self
                .files
                .libraries
                .file_library(target, &remote_path, true)
                .await?
                .map(|library| library.version)
                .or_else(|| {
                    remote
                        .data()
                        .and_then(|data| serialized_library_version(&data.content))
                });
            let (Some(local_version), Some(remote_version)) = (local_version, remote_version)
            else {
                // The catch-all around these reads also catches the NullReferenceException from a null Library.
                return Ok(RepositoryUpgrade::MalformedConfig);
            };

            return Ok(RepositoryUpgrade::Upgrade(
                compare_versions(&remote_version, &local_version).is_gt(),
            ));
        }

        Ok(RepositoryUpgrade::Upgrade(
            match (local.data(), remote.data()) {
                (Some(local), Some(remote)) => local.content != remote.content,
                _ => false,
            },
        ))
    }

    pub async fn check_apt_upgrade(
        &self,
        computer: ComputerId,
        file_path: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<AptUpgradeReport> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(AptUpgradeReport::new(Err(AptFailure::ComputerUnavailable)));
        }

        if !self
            .networks
            .reach
            .is_network_active(computer, terminal_computers)
            .await?
        {
            return Ok(AptUpgradeReport::new(Err(AptFailure::NoInternet)));
        }

        let content = match self.apt_cache_content(computer).await? {
            Ok(content) => content,
            Err(failure) => return Ok(AptUpgradeReport::new(Err(failure))),
        };

        if file_path.is_empty() {
            return Ok(AptUpgradeReport::new(Err(AptFailure::EmptyPackageName)));
        }

        if json_deserializes_to_null(&content) {
            return Err(WorldError::GameCrash);
        }

        let Ok(cache) = serde_json::from_str::<AptCache>(&content) else {
            return Ok(AptUpgradeReport::new(Err(AptFailure::MalformedCache)));
        };
        let name = path::file_name(file_path);

        if cache.official_server
            && !OFFICIAL_OMISSIONS.contains(&name)
            && let Some(latest) = self.official_library(name).await?
            && let Some(latest) = self.files.libraries.library_identity(latest).await?
        {
            if self
                .files
                .lookup
                .file(computer, file_path, true)
                .await?
                .is_none()
            {
                return Ok(AptUpgradeReport::new(Err(AptFailure::MissingLocalFile(
                    name.to_owned(),
                ))));
            }

            return Ok(AptUpgradeReport::new(Ok(self
                .files
                .libraries
                .file_library(computer, file_path, true)
                .await?
                .is_some_and(|library| {
                    compare_versions(&latest.version, &library.version).is_gt()
                }))));
        }

        let mut malformed_repositories = 0;

        for (address, server) in &cache.server_list {
            if !server.package_list.contains_key(name) {
                continue;
            }

            match self
                .check_repository_upgrade(
                    computer,
                    file_path,
                    name,
                    address,
                    server,
                    terminal_computers,
                )
                .await?
            {
                RepositoryUpgrade::Upgrade(upgrade) => {
                    return Ok(AptUpgradeReport {
                        result: Ok(upgrade),
                        malformed_repositories,
                    });
                }
                RepositoryUpgrade::Failure(failure) => {
                    return Ok(AptUpgradeReport {
                        result: Err(failure),
                        malformed_repositories,
                    });
                }
                RepositoryUpgrade::MalformedConfig => malformed_repositories += 1,
            }
        }

        Ok(AptUpgradeReport {
            result: Err(AptFailure::UnknownPackage(name.to_owned())),
            malformed_repositories,
        })
    }
}

/// Result of checking one repository source.
enum RepositoryUpgrade {
    Upgrade(bool),
    Failure(AptFailure),
    /// The catch-all around repository configuration and library reads printed the malformed-config line.
    MalformedConfig,
}

/// Reads the game's serialized `Library.version` without requiring unrelated weakness content.
fn serialized_library_version(content: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct SerializedLibrary {
        version: SerializedVersion,
    }

    #[derive(Deserialize)]
    struct SerializedVersion {
        version: [u32; 3],
    }

    let library = serde_json::from_str::<SerializedLibrary>(content).ok()?;
    let [major, minor, patch] = library.version.version;

    Some(format!("{major}.{minor}.{patch}"))
}

/// What `update` saved in `aptcache.bin` (`Service_repod.ClientCacheRepos`).
#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct AptCache {
    pub(crate) official_server: bool,
    #[serde(rename = "serverList")]
    pub(crate) server_list: IndexMap<String, CachedServer>,
}

/// A source's entry in the apt cache (`Service_repod.HelperClientCache`).
#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct CachedServer {
    #[serde(rename = "serverPort")]
    pub(crate) server_port: u16,
    #[serde(rename = "pkgList")]
    pub(crate) package_list: IndexMap<String, serde_json::Value>,
}
