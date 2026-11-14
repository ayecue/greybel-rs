//! Database connection, per-cycle transaction, and database copying.

use std::sync::Arc;

use turso::{Builder, Connection, params};

use crate::sql::execute;
use crate::{DomainSchema, SCHEMA_VERSIONS, Schema};

/// Authoritative database. Clones share one connection and its open transaction.
#[derive(Clone)]
pub struct Database {
    conn: Connection,
    tables: Arc<[&'static str]>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Database")
    }
}

impl Database {
    /// Opens an empty in-memory database with the tables of `schema`.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    pub async fn in_memory(schema: &Schema) -> turso::Result<Self> {
        Self::open(":memory:", schema).await
    }

    /// Opens or creates `path` with the tables of `schema` and foreign keys enabled, then runs the
    /// volatile statements of `schema` ([`Schema::volatile_sql`]). Web builds always use memory.
    ///
    /// # Errors
    ///
    /// Returns an error if the database or schema cannot be created.
    pub async fn open(path: &str, schema: &Schema) -> turso::Result<Self> {
        let builder = Builder::new_local(path);
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let builder = builder.with_io_impl(crate::web_io::WebIo::default_io());
        let conn = builder.build().await?.connect()?;

        conn.execute_batch("PRAGMA foreign_keys = ON;").await?;

        if schema.domains().is_empty() {
            conn.execute_batch(&schema.sql()).await?;
        } else {
            conn.execute_batch(SCHEMA_VERSIONS.schema()).await?;

            for domain in schema.domains() {
                open_domain(&conn, domain).await?;
            }

            for set in schema.ownerless_sets() {
                conn.execute_batch(set.schema()).await?;
            }
        }

        conn.execute_batch(&schema.volatile_sql()).await?;

        Ok(Self {
            conn,
            tables: schema.tables().into(),
        })
    }

    /// Returns the connection that statements run on.
    #[must_use]
    pub const fn connection(&self) -> &Connection {
        &self.conn
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

    /// Replaces all rows in `other` with this database's rows in one transaction on `other`. Both
    /// databases must have the schema this one was opened with.
    ///
    /// # Errors
    ///
    /// Returns any statement error from either database. `other` remains unchanged.
    pub async fn copy_into(&self, other: &Self) -> turso::Result<()> {
        other.begin().await?;

        let tables = &self.tables;
        let copy = async {
            for table in tables.iter().rev() {
                other
                    .conn
                    .execute(format!("DELETE FROM {table}"), ())
                    .await?;
            }

            for table in tables.iter() {
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

/// Creates or migrates one domain and records the schema version it reached.
async fn open_domain(conn: &Connection, domain: &DomainSchema) -> turso::Result<()> {
    let mut rows = conn
        .query(
            "SELECT version FROM schema_versions WHERE domain = ?1",
            params![domain.name],
        )
        .await?;
    let version = match rows.next().await? {
        Some(row) => {
            let version = row.get::<i64>(0)?;
            let version = u32::try_from(version)
                .ok()
                .filter(|version| *version > 0)
                .ok_or_else(|| {
                    turso::Error::ConversionFailure(format!(
                        "domain `{}` has invalid schema version {version}",
                        domain.name
                    ))
                })?;

            Some(version)
        }
        None => None,
    };

    drop(rows);

    if version.is_none() {
        for table in domain.tables.iter().flat_map(|set| set.tables()) {
            let mut table_rows = conn
                .query(
                    "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    params![table],
                )
                .await?;

            if table_rows.next().await?.is_some() {
                return Err(turso::Error::ConversionFailure(format!(
                    "domain `{}` has legacy table `{table}` but no schema version",
                    domain.name
                )));
            }
        }
    }

    if version.is_some_and(|version| version > domain.version) {
        return Err(turso::Error::ConversionFailure(format!(
            "domain `{}` has schema version {}, newer than supported version {}",
            domain.name,
            version.expect("checked as present"),
            domain.version
        )));
    }

    if version == Some(domain.version) {
        return create_tables(conn, domain).await;
    }

    execute(conn, "BEGIN", ()).await?;

    let result = async {
        if let Some(version) = version {
            let first = usize::try_from(version - 1).expect("version schema versions start at one");
            let migrations = domain.migrations.get(first..).ok_or_else(|| {
                turso::Error::ConversionFailure(format!(
                    "domain `{0}` has no migration from version {version} to {1}",
                    domain.name, domain.version
                ))
            })?;

            for migration in migrations {
                migration.step.run(conn).await.map_err(|error| {
                    turso::Error::ConversionFailure(format!(
                        "migration `{}` of domain `{}` failed: {error}",
                        migration.description, domain.name
                    ))
                })?;
            }
        }

        create_tables(conn, domain).await?;

        if version.is_some() {
            execute(
                conn,
                "UPDATE schema_versions SET version = ?2 WHERE domain = ?1",
                params![domain.name, i64::from(domain.version)],
            )
            .await?;
        } else {
            execute(
                conn,
                "INSERT INTO schema_versions (domain, version) VALUES (?1, ?2)",
                params![domain.name, i64::from(domain.version)],
            )
            .await?;
        }

        conn.execute_batch("COMMIT").await
    }
    .await;

    if result.is_err() {
        let _ = conn.execute_batch("ROLLBACK").await;
    }

    result
}

async fn create_tables(conn: &Connection, domain: &DomainSchema) -> turso::Result<()> {
    for set in &domain.tables {
        conn.execute_batch(set.schema()).await?;
    }

    Ok(())
}
