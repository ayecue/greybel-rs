//! World database connection, per-cycle transaction, and database copying.

use turso::{Builder, Connection};

use super::schema::{VOLATILE, schema, tables};
use super::sql::execute;

/// Authoritative world database. Clones share one connection and its open transaction.
#[derive(Clone)]
pub struct Database {
    pub(in crate::storage) conn: Connection,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Database")
    }
}

impl Database {
    /// Opens an empty in-memory database with the world schema.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    pub async fn in_memory() -> turso::Result<Self> {
        Self::open(":memory:").await
    }

    /// Opens or creates `path` with the world schema and foreign keys enabled, deleting volatile
    /// program state ([`VOLATILE`]). Web builds always use memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    pub async fn open(path: &str) -> turso::Result<Self> {
        let builder = Builder::new_local(path);
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let builder = builder.with_io_impl(super::web_io::WebIo::default_io());
        let conn = builder.build().await?.connect()?;

        conn.execute_batch("PRAGMA foreign_keys = ON;").await?;
        conn.execute_batch(&schema()).await?;
        conn.execute_batch(VOLATILE).await?;

        Ok(Self { conn })
    }

    /// Begins the per-cycle read/write transaction, dropping any uncommitted transaction left by an
    /// earlier cycle.
    ///
    /// # Errors
    ///
    /// Returns an error if Turso cannot begin or drop a transaction.
    pub async fn begin(&self) -> turso::Result<()> {
        self.rollback().await?;
        execute(&self.conn, "BEGIN", ()).await?;

        Ok(())
    }

    /// Commits the open transaction. Does nothing if no transaction is open.
    ///
    /// # Errors
    ///
    /// Returns a Turso commit error. The transaction is dropped.
    pub async fn commit(&self) -> turso::Result<()> {
        if self.conn.is_autocommit()? {
            return Ok(());
        }

        if let Err(error) = execute(&self.conn, "COMMIT", ()).await {
            // Report the commit error. After rollback failure, the next `begin` drops the
            // transaction.
            let _ = self.rollback().await;

            return Err(error);
        }

        Ok(())
    }

    /// Rolls back and drops the open transaction. Does nothing if no transaction is open.
    ///
    /// # Errors
    ///
    /// Returns an error if Turso cannot drop the transaction.
    pub async fn rollback(&self) -> turso::Result<()> {
        if !self.conn.is_autocommit()? {
            execute(&self.conn, "ROLLBACK", ()).await?;
        }

        Ok(())
    }

    /// Returns whether a transaction is open.
    ///
    /// # Errors
    ///
    /// Returns an error if Turso cannot query transaction state.
    pub fn in_transaction(&self) -> turso::Result<bool> {
        Ok(!self.conn.is_autocommit()?)
    }

    /// Replaces all rows in `other` with this database's rows in one transaction on `other`.
    ///
    /// # Errors
    ///
    /// Returns any statement error from either database. `other` remains unchanged.
    pub async fn copy_into(&self, other: &Self) -> turso::Result<()> {
        other.begin().await?;

        let tables = tables();
        let copy = async {
            for table in tables.iter().rev() {
                other
                    .conn
                    .execute(format!("DELETE FROM {table}"), ())
                    .await?;
            }

            for table in &tables {
                let mut rows = self
                    .conn
                    .query(format!("SELECT * FROM {table}"), ())
                    .await?;
                let mut insert = None;

                while let Some(row) = rows.next().await? {
                    let count = row.column_count();
                    let values = (0..count)
                        .map(|index| row.get_value(index))
                        .collect::<turso::Result<Vec<_>>>()?;
                    let statement = insert.get_or_insert_with(|| {
                        let marks = (1..=count)
                            .map(|index| format!("?{index}"))
                            .collect::<Vec<_>>()
                            .join(", ");

                        format!("INSERT INTO {table} VALUES ({marks})")
                    });

                    execute(&other.conn, statement, values).await?;
                }
            }

            Ok(())
        }
        .await;

        match copy {
            Ok(()) => other.commit().await,
            Err(error) => {
                let _ = other.rollback().await;

                Err(error)
            }
        }
    }
}
