//! Package repository rows owned by the services domain.

use greyhack_behaviour::{FIRST_VERSION, LibraryKind};
use greyhack_world_core::{Database, Result};
use greyhack_world_storage::sql::{
    Code, FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value,
};
use turso::params;

use crate::{Package, Repository};

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

/// Queries of the package-repository tables. Library entities are resolved by the libraries service.
pub(crate) struct AptRepository {
    db: Database,
}

impl AptRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn has_repository(&self, address: &str) -> Result<bool> {
        Ok(fetch_value::<i64>(
            &self.db.connection(),
            "SELECT 1 FROM repositories WHERE address = ?1",
            params![address],
        )
        .await?
        .is_some())
    }

    pub(crate) async fn repository_rows(
        &self,
        address: &str,
    ) -> Result<Vec<(String, String, Option<Code<LibraryKind>>)>> {
        Ok(fetch_all(
            &self.db.connection(),
            "SELECT name, description, library FROM repository_packages WHERE address = ?1 ORDER BY id",
            params![address],
        )
        .await?)
    }

    pub(crate) async fn package_row(
        &self,
        address: &str,
        name: &str,
    ) -> Result<Option<(String, String, Option<Code<LibraryKind>>)>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT name, description, library FROM repository_packages WHERE address = ?1 AND name = ?2",
            params![address, name],
        )
        .await?)
    }

    pub(crate) async fn insert_repository_row(&self, repository: &Repository) -> Result<bool> {
        if self.has_repository(&repository.address).await? {
            return Ok(false);
        }

        execute(
            &self.db.connection(),
            "INSERT INTO repositories (address) VALUES (?1)",
            params![repository.address.as_str()],
        )
        .await?;

        Ok(true)
    }

    pub(crate) async fn insert_package(
        &self,
        address: &str,
        package: &Package,
        kind: Option<LibraryKind>,
    ) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO repository_packages (address, name, description, library) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                address,
                package.name.as_str(),
                package.description.as_str(),
                kind.map(LibraryKind::code)
            ],
        )
        .await?;

        Ok(())
    }
}
