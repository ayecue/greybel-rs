//! Package repository and package storage.

use greyhack_behaviour::{FIRST_VERSION, LibraryKind};
use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value};
use crate::{Package, Repository, Result};

impl FromRow for Package {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            name: row.get("name")?,
            version: row
                .get::<Option<String>>("version")?
                .unwrap_or_else(|| FIRST_VERSION.to_owned()),
            description: row.get("description")?,
            library: None,
        })
    }
}

impl Database {
    /// Returns whether repository `address` exists.
    pub(crate) async fn has_repository(&self, address: &str) -> Result<bool> {
        Ok(fetch_value::<i64>(
            &self.conn,
            "SELECT 1 FROM repositories WHERE address = ?1",
            params![address],
        )
        .await?
        .is_some())
    }

    /// Returns repository `address` with packages in insertion order, excluding library contents.
    pub(crate) async fn repository(&self, address: &str) -> Result<Option<Repository>> {
        if !self.has_repository(address).await? {
            return Ok(None);
        }

        let packages = fetch_all(
            &self.conn,
            "SELECT repository_packages.name, libraries.version, repository_packages.description \
             FROM repository_packages LEFT JOIN library_latest ON library_latest.kind = repository_packages.library \
             LEFT JOIN libraries ON libraries.id = library_latest.library_id \
             WHERE repository_packages.address = ?1 ORDER BY repository_packages.id",
            params![address],
        )
        .await?;

        Ok(Some(Repository {
            address: address.to_owned(),
            packages,
        }))
    }

    /// Returns package `name` from repository `address` with the newest library version.
    pub(crate) async fn package(&self, address: &str, name: &str) -> Result<Option<Package>> {
        let row: Option<(String, String, Option<LibraryKind>)> = fetch_opt(
            &self.conn,
            "SELECT name, description, library FROM repository_packages WHERE address = ?1 AND name = ?2",
            params![address, name],
        )
        .await?;
        let Some((name, description, kind)) = row else {
            return Ok(None);
        };
        let library = match kind {
            Some(kind) => match self.latest_library(kind).await? {
                Some(id) => self.library(id).await?.map(Box::new),
                None => None,
            },
            None => None,
        };

        Ok(Some(Package {
            name,
            version: library.as_ref().map_or_else(
                || FIRST_VERSION.to_owned(),
                |library| library.version.clone(),
            ),
            description,
            library,
        }))
    }

    /// Inserts `repository` and its packages, storing libraries by kind. Does nothing if it exists.
    pub(crate) async fn insert_repository(&self, repository: &Repository) -> Result<()> {
        if self.has_repository(&repository.address).await? {
            return Ok(());
        }

        execute(
            &self.conn,
            "INSERT INTO repositories (address) VALUES (?1)",
            params![repository.address.as_str()],
        )
        .await?;

        for package in &repository.packages {
            let kind = match &package.library {
                Some(library) => {
                    self.library_id(library).await?;

                    Some(library.kind)
                }
                None => None,
            };

            execute(
                &self.conn,
                "INSERT OR REPLACE INTO repository_packages (address, name, description, library) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    repository.address.as_str(),
                    package.name.as_str(),
                    package.description.as_str(),
                    kind.map(LibraryKind::code)
                ],
            )
            .await?;
        }

        Ok(())
    }
}
