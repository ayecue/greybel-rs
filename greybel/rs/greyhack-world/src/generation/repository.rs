//! Builds the default `hackshop.org` repository using the world seed.

use greyhack_behaviour::{DEFAULT_REPOSITORY, FIRST_VERSION, LibraryKind};

use crate::{Library, Package, Repository, Rng};

impl Repository {
    /// Generates default `hackshop.org` packages, including `nmap` and exploitation libraries with
    /// weakness data.
    #[must_use]
    pub fn default_repository(rng: &mut Rng) -> Self {
        let library = |rng: &mut Rng, kind: LibraryKind, description: &str| Package {
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
                Package {
                    name: "nmap".to_owned(),
                    version: FIRST_VERSION.to_owned(),
                    description: "Scan a computer's open ports.".to_owned(),
                    library: None,
                },
            ],
        }
    }
}
