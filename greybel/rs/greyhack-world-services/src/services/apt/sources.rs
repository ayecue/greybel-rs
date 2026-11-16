//! Package sources: repositories, the sources file, and the official cache.

use greyhack_behaviour::{
    APT_CACHE_NAME, APT_CACHE_PATH, APT_SOURCES_PATH, AptSource, DOWNLOADS_PATH, FIRST_VERSION,
    FileKind, ServiceId, format_apt_sources, is_valid_ip, parse_apt_official, parse_apt_sources,
};
use greyhack_world_core::systems::network::connect::ServiceCheck;
use greyhack_world_core::{ComputerId, Result};

use crate::services::Apt;
use crate::{AptFailure, Package, Repository, RepositoryConfig};

/// Methods of [`World`] for services apt sources.
impl Apt {
    pub async fn repository(&self, address: &str) -> Result<Option<Repository>> {
        if !self.repositories.has_repository(address).await? {
            return Ok(None);
        }

        let mut packages = Vec::new();

        for (name, description, kind) in self.repositories.repository_rows(address).await? {
            let mut version = FIRST_VERSION.to_owned();

            if let Some(kind) = kind.map(|greyhack_world_storage::sql::Code(kind)| kind)
                && let Some(id) = self.files.latest_library(kind).await?
                && let Some(library) = self.files.libraries.library_identity(id).await?
            {
                version = library.version;
            }

            packages.push(Package {
                name,
                version,
                description,
                library: None,
            });
        }

        Ok(Some(Repository {
            address: address.to_owned(),
            packages,
        }))
    }

    pub async fn package(&self, address: &str, name: &str) -> Result<Option<Package>> {
        let Some((name, description, kind)) = self.repositories.package_row(address, name).await?
        else {
            return Ok(None);
        };
        let mut library = None;
        let mut version = FIRST_VERSION.to_owned();

        if let Some(kind) = kind.map(|greyhack_world_storage::sql::Code(kind)| kind)
            && let Some(id) = self.files.latest_library(kind).await?
            && let Some(identity) = self.files.libraries.library_identity(id).await?
        {
            version = identity.version;
            library = Some(id);
        }

        Ok(Some(Package {
            name,
            version,
            description,
            library,
        }))
    }

    pub async fn apt_sources(&self, computer: ComputerId) -> Result<Vec<AptSource>> {
        Ok(self
            .files
            .lookup
            .content(computer, APT_SOURCES_PATH, true)
            .await?
            .map_or_else(Vec::new, |content| parse_apt_sources(&content)))
    }

    pub async fn apt_cache_content(
        &self,
        computer: ComputerId,
    ) -> Result<Result<String, AptFailure>> {
        let Some(cache) = self
            .files
            .lookup
            .file(computer, APT_CACHE_PATH, true)
            .await?
        else {
            return Ok(Err(AptFailure::NoCache));
        };
        let Some(data) = cache.data() else {
            return Ok(Err(AptFailure::InvalidCache));
        };

        if data.command_name != APT_CACHE_NAME {
            return Ok(Err(AptFailure::InvalidCache));
        }

        Ok(Ok(data.content.clone()))
    }

    pub async fn apt_cache_official(
        &self,
        computer: ComputerId,
    ) -> Result<Result<bool, AptFailure>> {
        Ok(self.apt_cache_content(computer).await?.and_then(|content| {
            serde_json::from_str::<serde_json::Value>(&content)
                .map_err(|_| AptFailure::MalformedCache)
                .map(|_| parse_apt_official(&content))
        }))
    }

    pub async fn apt_official(&self, computer: ComputerId) -> Result<bool> {
        Ok(self
            .files
            .lookup
            .content(computer, APT_SOURCES_PATH, true)
            .await?
            .is_some_and(|content| parse_apt_official(&content)))
    }

    pub async fn source_address(&self, source: &AptSource) -> Result<Option<String>> {
        if is_valid_ip(&source.address) {
            return Ok(Some(source.address.clone()));
        }

        self.networks.routers.lookup(&source.address).await
    }

    pub async fn remote_repository(
        &self,
        from: ComputerId,
        source: &AptSource,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<(ComputerId, RepositoryConfig), AptFailure>> {
        let Some(address) = self.source_address(source).await? else {
            return Ok(Err(AptFailure::UnknownRepository(source.address.clone())));
        };
        let target = match self
            .networks
            .connect
            .connect_computer(from, &address, source.port)
            .await?
        {
            Ok(target) => target,
            Err(failure) => return Ok(Err(AptFailure::Connection(failure))),
        };
        let public = self.networks.topology.public_ip(from).await?;

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
            return Ok(Err(AptFailure::Service(failure)));
        }

        let Some(node) = self
            .files
            .lookup
            .file(target, ServiceId::Repository.config_path(), true)
            .await?
        else {
            return Ok(Err(AptFailure::MissingConfig));
        };
        let Some(data) = node.data() else {
            return Ok(Err(AptFailure::MissingConfig));
        };
        let Ok(config) = serde_json::from_str::<RepositoryConfig>(&data.content) else {
            return Ok(Err(AptFailure::MalformedConfig));
        };

        Ok(Ok((target, config)))
    }

    pub async fn source_package(
        &self,
        computer: ComputerId,
        source: &AptSource,
        name: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Result<(ComputerId, String), AptFailure>> {
        let (server, config) = match self
            .remote_repository(computer, source, terminal_computers)
            .await?
        {
            Ok(repository) => repository,
            Err(failure) => return Ok(Err(failure)),
        };

        self.installs
            .update_hackshop_libs(server, DOWNLOADS_PATH)
            .await?;

        Ok(Ok((server, format!("{}/{name}", config.path_repo))))
    }

    async fn validate_sources_for_edit(&self, computer: ComputerId) -> Result<Option<AptFailure>> {
        let Some(node) = self
            .files
            .lookup
            .file(computer, APT_SOURCES_PATH, true)
            .await?
        else {
            return Ok(Some(AptFailure::MissingSources));
        };
        let Some(data) = node.data() else {
            return Ok(Some(AptFailure::BinarySources));
        };

        if data.kind == FileKind::Binary {
            return Ok(Some(AptFailure::BinarySources));
        }

        if serde_json::from_str::<serde_json::Value>(&data.content).is_err() {
            return Ok(Some(AptFailure::MalformedSources));
        }

        Ok(None)
    }

    pub async fn add_apt_repository(
        &self,
        computer: ComputerId,
        repository: &str,
        port: u16,
    ) -> Result<Result<(), AptFailure>> {
        if let Some(failure) = self.validate_sources_for_edit(computer).await? {
            return Ok(Err(failure));
        }

        if !is_valid_ip(repository) {
            return Ok(Err(AptFailure::InvalidAddress(repository.to_owned())));
        }

        let mut repositories = self.apt_sources(computer).await?;

        if repositories.iter().any(|repo| repo.address == repository) {
            return Ok(Err(AptFailure::ListedRepository(repository.to_owned())));
        }

        let official = self.apt_official(computer).await?;

        repositories.push(AptSource {
            address: repository.to_owned(),
            port,
        });
        self.write_apt_sources(computer, &repositories, official)
            .await?;

        Ok(Ok(()))
    }

    pub async fn remove_apt_repository(
        &self,
        computer: ComputerId,
        repository: &str,
    ) -> Result<Result<(), AptFailure>> {
        if let Some(failure) = self.validate_sources_for_edit(computer).await? {
            return Ok(Err(failure));
        }

        let mut repositories = self.apt_sources(computer).await?;

        if !repositories.iter().any(|repo| repo.address == repository) {
            return Ok(Err(AptFailure::UnlistedRepository(repository.to_owned())));
        }

        let official = self.apt_official(computer).await?;

        repositories.retain(|repo| repo.address != repository);
        self.write_apt_sources(computer, &repositories, official)
            .await?;

        Ok(Ok(()))
    }
}

impl Apt {
    pub async fn write_apt_sources(
        &self,
        computer: ComputerId,
        repositories: &[AptSource],
        official: bool,
    ) -> Result<()> {
        if let Some(mut node) = self
            .files
            .lookup
            .file(computer, APT_SOURCES_PATH, true)
            .await?
        {
            if let Some(data) = node.data_mut() {
                data.set_content(&format_apt_sources(repositories, official));
            }

            self.files.changes.save_node(&node).await?;
        }

        Ok(())
    }
}
