//! The world database: the generic database opened with the schema its domains register.

use greyhack_world_storage::{Database as Base, Schema};
use turso::Connection;

/// Authoritative world database. Clones share one connection and its open transaction. DomainSet
/// add their queries to it with storage traits that use [`Database::conn`].
#[derive(Clone)]
pub struct Database {
    base: Base,
    /// The connection statements run on.
    pub conn: Connection,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Database")
    }
}

impl Database {
    /// Opens an empty in-memory database with `schema`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    pub async fn in_memory_with(schema: &Schema) -> turso::Result<Self> {
        Self::open_with(":memory:", schema).await
    }

    /// Opens or creates `path` with `schema` and foreign keys enabled, deleting volatile program
    /// state. Web builds always use memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    pub async fn open_with(path: &str, schema: &Schema) -> turso::Result<Self> {
        let base = Base::open(path, schema).await?;
        let conn = base.connection().clone();

        Ok(Self { base, conn })
    }

    /// Returns the connection that statements run on.
    pub fn connection(&self) -> Connection {
        self.conn.clone()
    }

    /// Begins the per-cycle read/write transaction, dropping any uncommitted transaction left by an
    /// earlier cycle.
    ///
    /// # Errors
    ///
    /// Returns an error if Turso cannot begin or drop a transaction.
    pub async fn begin(&self) -> turso::Result<()> {
        self.base.begin().await
    }

    /// Commits the open transaction. Does nothing if no transaction is open.
    ///
    /// # Errors
    ///
    /// Returns a Turso commit error. The transaction is dropped.
    pub async fn commit(&self) -> turso::Result<()> {
        self.base.commit().await
    }

    /// Rolls back and drops the open transaction. Does nothing if no transaction is open.
    ///
    /// # Errors
    ///
    /// Returns an error if Turso cannot drop the transaction.
    pub async fn rollback(&self) -> turso::Result<()> {
        self.base.rollback().await
    }

    /// Returns whether a transaction is open.
    ///
    /// # Errors
    ///
    /// Returns an error if Turso cannot query transaction state.
    pub fn in_transaction(&self) -> turso::Result<bool> {
        self.base.in_transaction()
    }

    /// Replaces all rows in `other` with this database's rows in one transaction on `other`.
    ///
    /// # Errors
    ///
    /// Returns any statement error from either database. `other` remains unchanged.
    pub async fn copy_into(&self, other: &Self) -> turso::Result<()> {
        self.base.copy_into(&other.base).await
    }
}
