//! Opening the world database with the schema of every domain.

use greyhack_world_core::Database;

use super::schema::world_schema;

/// Constructors of [`Database`] with the world schema: the core only knows its own tables.
pub trait DatabaseOpen: Sized {
    /// Opens an empty in-memory database with the world schema.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    async fn in_memory() -> turso::Result<Self>;

    /// Opens or creates `path` with the world schema and foreign keys enabled, deleting volatile
    /// program state. Web builds always use memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    async fn open(path: &str) -> turso::Result<Self>;
}

impl DatabaseOpen for Database {
    async fn in_memory() -> turso::Result<Self> {
        Self::open(":memory:").await
    }

    async fn open(path: &str) -> turso::Result<Self> {
        Self::open_with(path, &world_schema()).await
    }
}
