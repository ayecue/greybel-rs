//! Builds the default `hackshop.org` repository using the world seed.

use greyhack_behaviour::{DEFAULT_REPOSITORY, FIRST_VERSION, LibraryKind, Rng};
use greyhack_world_core::Result;
use greyhack_world_libraries::{Generator, Library, LibraryGeneration as _};
use greyhack_world_services::{Package, Repository};

/// Default repository whose generated libraries still need persistent identities.
pub struct RepositoryDraft {
    address: String,
    packages: Vec<PackageDraft>,
}

struct PackageDraft {
    name: String,
    version: String,
    description: String,
    library: Option<Box<Library>>,
}

impl RepositoryDraft {
    /// Generates default `hackshop.org` packages, including `nmap` and exploitation libraries with
    /// weakness data.
    #[must_use]
    pub fn default_repository(rng: &mut Rng) -> Self {
        let library = |rng: &mut Rng, kind: LibraryKind, description: &str| PackageDraft {
            name: kind.file_name(),
            version: FIRST_VERSION.to_owned(),
            description: description.to_owned(),
            library: Some(Box::new(Library::generate(rng, kind, FIRST_VERSION))),
        };

        Self {
            address: DEFAULT_REPOSITORY.to_owned(),
            packages: vec![
                library(
                    rng,
                    LibraryKind::Metaxploit,
                    "The essential tool for every security researcher.",
                ),
                library(
                    rng,
                    LibraryKind::Crypto,
                    "Password and encryption utilities.",
                ),
                library(rng, LibraryKind::Blockchain, "Work with coins and wallets."),
                PackageDraft {
                    name: "nmap".to_owned(),
                    version: FIRST_VERSION.to_owned(),
                    description: "Scan a computer's open ports.".to_owned(),
                    library: None,
                },
            ],
        }
    }

    /// Stores generated libraries and converts the draft to its ID-based repository entity.
    pub async fn store(self, generator: &Generator) -> Result<Repository> {
        let mut packages = Vec::new();

        for package in self.packages {
            let library = match package.library {
                Some(library) => Some(generator.library_id(&library).await?),
                None => None,
            };

            packages.push(Package {
                name: package.name,
                version: package.version,
                description: package.description,
                library,
            });
        }

        Ok(Repository {
            address: self.address,
            packages,
        })
    }
}
