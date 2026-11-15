//! Package sources, installation and searches, and updates.

pub(crate) mod packages;
pub(crate) mod sources;
pub(crate) mod updates;

use greyhack_world_core::Result;

use crate::Repository;

/// A world service with only its declared dependencies.
pub struct Apt {
    pub(crate) repositories: std::rc::Rc<crate::storage::AptRepository>,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) installs: greyhack_world_modules::Dep<crate::services::InstallService>,
}

impl Apt {
    pub(crate) fn new(
        repositories: std::rc::Rc<crate::storage::AptRepository>,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        installs: greyhack_world_modules::Dep<crate::services::InstallService>,
    ) -> Self {
        Self {
            repositories,
            computers,
            files,
            networks,
            installs,
        }
    }
}

impl Apt {
    /// Stores a repository whose package libraries already have stable identities.
    pub async fn insert_repository(&self, repository: &Repository) -> Result<()> {
        if !self.repositories.insert_repository_row(repository).await? {
            return Ok(());
        }

        for package in &repository.packages {
            let kind = match package.library {
                Some(id) => Some(
                    self.files
                        .libraries
                        .library_identity(id)
                        .await?
                        .expect("package libraries are stored")
                        .kind,
                ),
                None => None,
            };

            self.repositories
                .insert_package(&repository.address, package, kind)
                .await?;
        }

        Ok(())
    }
}
