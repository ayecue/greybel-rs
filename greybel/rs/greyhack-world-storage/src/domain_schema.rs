//! Domain-owned schemas and their migrations.

use std::future::Future;
use std::pin::Pin;

use turso::Connection;

use crate::TableSet;

/// Future returned by a Rust migration step.
pub type MigrationFuture<'a> = Pin<Box<dyn Future<Output = turso::Result<()>> + 'a>>;

/// One change between consecutive schema versions of a domain.
#[derive(Clone, Copy)]
pub struct Migration {
    /// What the step changes, for errors and logs.
    pub description: &'static str,
    /// SQL statements, or a Rust step for changes SQL cannot express.
    pub step: MigrationStep,
}

impl std::fmt::Debug for Migration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Migration")
            .field("description", &self.description)
            .finish_non_exhaustive()
    }
}

/// Work performed by one migration.
#[derive(Clone, Copy)]
pub enum MigrationStep {
    /// Idempotent SQL statements to execute.
    Sql(&'static str),
    /// Data rewrite or another migration SQL alone cannot express.
    Rust(for<'a> fn(&'a Connection) -> MigrationFuture<'a>),
}

impl std::fmt::Debug for MigrationStep {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sql(sql) => formatter.debug_tuple("Sql").field(sql).finish(),
            Self::Rust(_) => formatter.write_str("Rust(..)"),
        }
    }
}

impl MigrationStep {
    pub(crate) async fn run(&self, conn: &Connection) -> turso::Result<()> {
        match self {
            Self::Sql(sql) => conn.execute_batch(sql).await,
            Self::Rust(step) => step(conn).await,
        }
    }
}

/// Schema of one domain at its current version.
#[derive(Debug, Clone)]
pub struct DomainSchema {
    /// Domain name used as its key in `schema_versions`.
    pub name: &'static str,
    /// Names of domains whose tables this domain's foreign keys reference directly.
    pub depends_on: &'static [&'static str],
    /// The domain's table sets at `version`, in creation order.
    pub tables: Vec<TableSet>,
    /// Schema version of this build of the domain.
    pub version: u32,
    /// Steps from each older version to the next: `migrations[0]` takes version 1 to 2.
    pub migrations: &'static [Migration],
}

impl DomainSchema {
    /// Creates the version-one schema of `name` from its table sets.
    #[must_use]
    pub fn new(name: &'static str, tables: impl IntoIterator<Item = TableSet>) -> Self {
        Self {
            name,
            depends_on: &[],
            tables: tables.into_iter().collect(),
            version: 1,
            migrations: &[],
        }
    }

    /// Declares the domains whose tables this domain's foreign keys reference directly.
    #[must_use]
    pub fn with_dependencies(mut self, depends_on: &'static [&'static str]) -> Self {
        self.depends_on = depends_on;

        self
    }

    /// Sets the current version and migration path of this schema.
    #[must_use]
    pub fn with_migrations(mut self, version: u32, migrations: &'static [Migration]) -> Self {
        assert!(
            version > 0 && usize::try_from(version - 1).ok() == Some(migrations.len()),
            "a domain needs one migration per version step"
        );
        self.version = version;
        self.migrations = migrations;

        self
    }
}

/// Versions of domain schemas persisted in a world.
pub const SCHEMA_VERSIONS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS schema_versions (
    domain  TEXT PRIMARY KEY,
    version INTEGER NOT NULL
);

",
    &["schema_versions"],
);
