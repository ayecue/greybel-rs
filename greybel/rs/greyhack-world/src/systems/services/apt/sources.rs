//! Package sources: repositories, the sources file, and the official cache.

use greyhack_behaviour::{
    APT_CACHE_NAME, APT_CACHE_PATH, APT_SOURCES_PATH, AptSource, DOWNLOADS_PATH, ServiceId,
    format_apt_sources, is_lan_ip, is_valid_ip, parse_apt_official, parse_apt_sources,
};

use crate::systems::network::connect::ServiceCheck;
use crate::{AptFailure, ComputerId, Package, Repository, RepositoryConfig, Result, World};

impl World {
    /// Returns repository `address` and packages without library contents.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn repository(&self, address: &str) -> Result<Option<Repository>> {
        self.db.repository(address).await
    }

    /// Returns package `name` from repository `address`, including its library.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn package(&self, address: &str, name: &str) -> Result<Option<Package>> {
        self.db.package(address, name).await
    }

    /// Returns repository addresses from the computer `sources.txt`.
    pub(super) async fn apt_sources(&self, computer: ComputerId) -> Result<Vec<AptSource>> {
        Ok(self
            .content(computer, APT_SOURCES_PATH, true)
            .await?
            .map_or_else(Vec::new, |content| parse_apt_sources(&content)))
    }

    pub(super) async fn apt_cache_official(
        &self,
        computer: ComputerId,
    ) -> Result<Result<bool, AptFailure>> {
        let Some(cache) = self.file(computer, APT_CACHE_PATH, true).await? else {
            return Ok(Err(AptFailure::NoCache));
        };
        let Some(data) = cache.data() else {
            return Ok(Err(AptFailure::InvalidCache));
        };

        if data.command_name != APT_CACHE_NAME {
            return Ok(Err(AptFailure::InvalidCache));
        }

        Ok(Ok(parse_apt_official(&data.content)))
    }

    pub(super) async fn apt_official(&self, computer: ComputerId) -> Result<bool> {
        Ok(self
            .content(computer, APT_SOURCES_PATH, true)
            .await?
            .is_some_and(|content| parse_apt_official(&content)))
    }

    /// Returns the address of `source`: its own when it is an address, otherwise the address its
    /// domain resolves to.
    pub(super) async fn source_address(&self, source: &AptSource) -> Result<Option<String>> {
        if is_valid_ip(&source.address) {
            return Ok(Some(source.address.clone()));
        }

        self.lookup(&source.address).await
    }

    /// Returns the repository for `source` using its service-published catalog.
    pub(super) async fn remote_repository(
        &self,
        from: ComputerId,
        source: &AptSource,
        terminal_computers: &[ComputerId],
    ) -> Result<Option<(ComputerId, RepositoryConfig)>> {
        let Some(address) = self.source_address(source).await? else {
            return Ok(None);
        };
        let target = if is_lan_ip(&address) {
            match self.router_of(from).await? {
                Some(router) => self.device(router.network, &address).await?,
                None => None,
            }
        } else {
            let Some(router) = self.router_at(&address).await? else {
                return Ok(None);
            };
            let table = self.port_table(router.id).await?;
            let Some(entry) = table.iter().find(|entry| entry.number == source.port) else {
                return Ok(None);
            };

            self.device(router.network, &entry.lan_ip).await?
        };
        let Some(target) = target else {
            return Ok(None);
        };
        let public = self.public_ip(from).await?;

        if self
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
            .is_some()
        {
            return Ok(None);
        }

        let Some(content) = self
            .content(target, ServiceId::Repository.config_path(), true)
            .await?
        else {
            return Ok(None);
        };
        let Ok(config) = serde_json::from_str::<RepositoryConfig>(&content) else {
            return Ok(None);
        };

        Ok(Some((target, config)))
    }

    /// Returns the repository server and the path of package `name` on it when `source` lists the
    /// package, after the server refreshes its shop libraries.
    pub(super) async fn source_package(
        &mut self,
        computer: ComputerId,
        source: &AptSource,
        name: &str,
        terminal_computers: &[ComputerId],
    ) -> Result<Option<(ComputerId, String)>> {
        let Some((server, config)) = self
            .remote_repository(computer, source, terminal_computers)
            .await?
        else {
            return Ok(None);
        };

        if !config.repo_sources.contains_key(name) {
            return Ok(None);
        }

        self.update_hackshop_libs(server, DOWNLOADS_PATH).await?;

        Ok(Some((server, format!("{}/{name}", config.path_repo))))
    }

    /// Rewrites computer `sources.txt` from `repositories`.
    async fn write_apt_sources(
        &mut self,
        computer: ComputerId,
        repositories: &[AptSource],
        official: bool,
    ) -> Result<()> {
        if let Some(mut node) = self.file(computer, APT_SOURCES_PATH, true).await? {
            if let Some(data) = node.data_mut() {
                data.set_content(&format_apt_sources(repositories, official));
            }

            self.save_node(&node).await?;
        }

        Ok(())
    }

    /// Adds an IP repository and port to `sources.txt` (`AptClientIntrinsics.add_repo`). Does not
    /// connect until `update`.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_apt_repository(
        &mut self,
        computer: ComputerId,
        repository: &str,
        port: u16,
    ) -> Result<Result<(), AptFailure>> {
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

    /// Removes `repository` from `sources.txt` or returns the failure.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn remove_apt_repository(
        &mut self,
        computer: ComputerId,
        repository: &str,
    ) -> Result<Result<(), AptFailure>> {
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
