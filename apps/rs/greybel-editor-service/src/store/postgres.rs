use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

use super::{SaveReport, StoreError, WorkspaceRecord, WorkspaceStore};
use crate::workspace::{LegacyContent, Workspace};

/// Workspaces in Postgres: the `workspace` and `workspace_file` tables, and greybel-editor-be's `"Code"` table
/// when the database has it.
#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
    has_legacy_table: bool,
}

impl PostgresStore {
    /// Connects and applies the migrations; logs the host and database, not the credentials.
    pub async fn connect(url: &str, max_connections: u32) -> Result<Self, StoreError> {
        let options: PgConnectOptions = url.parse()?;
        let host = options.get_host().to_owned();
        let database = options.get_database().unwrap_or_default().to_owned();
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;

        sqlx::migrate!().run(&pool).await?;

        let has_legacy_table: bool =
            sqlx::query_scalar("SELECT to_regclass('\"Code\"') IS NOT NULL")
                .fetch_one(&pool)
                .await?;

        tracing::info!(
            %host,
            %database,
            max_connections,
            legacy_table = has_legacy_table,
            "connected to Postgres, migrations applied"
        );

        Ok(Self {
            pool,
            has_legacy_table,
        })
    }

    async fn find_workspace(&self, id: Uuid) -> Result<Option<WorkspaceRecord>, StoreError> {
        let Some((entry, creation_time)) = sqlx::query_as::<_, (String, DateTime<Utc>)>(
            "SELECT entry, creation_time FROM workspace WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };
        let files: Vec<(String, String)> =
            sqlx::query_as("SELECT path, content FROM workspace_file WHERE workspace_id = $1")
                .bind(id)
                .fetch_all(&self.pool)
                .await?;

        Ok(Some(WorkspaceRecord {
            id,
            workspace: Workspace {
                entry,
                files: files.into_iter().collect::<BTreeMap<_, _>>(),
            },
            creation_time,
        }))
    }

    async fn find_legacy(&self, id: Uuid) -> Result<Option<WorkspaceRecord>, StoreError> {
        if !self.has_legacy_table {
            return Ok(None);
        }

        let row: Option<(String, NaiveDateTime)> =
            sqlx::query_as("SELECT content, \"createdAt\" FROM \"Code\" WHERE id = $1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        let Some((content, creation_time)) = row else {
            return Ok(None);
        };

        tracing::info!(%id, "read a project of the legacy \"Code\" table");

        Ok(Some(WorkspaceRecord {
            id,
            workspace: LegacyContent::decode(&content),
            creation_time: creation_time.and_utc(),
        }))
    }
}

impl WorkspaceStore for PostgresStore {
    async fn save(&self, workspace: Workspace) -> Result<SaveReport, StoreError> {
        let hash = workspace.content_hash();
        let mut transaction = self.pool.begin().await?;
        let new_row: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
            "INSERT INTO workspace (id, entry, content_hash) VALUES ($1, $2, $3) \
             ON CONFLICT (content_hash) DO NOTHING RETURNING id, creation_time",
        )
        .bind(Uuid::new_v4())
        .bind(&workspace.entry)
        .bind(&hash)
        .fetch_optional(&mut *transaction)
        .await?;

        let Some((id, creation_time)) = new_row else {
            transaction.rollback().await?;

            let (id, creation_time): (Uuid, DateTime<Utc>) =
                sqlx::query_as("SELECT id, creation_time FROM workspace WHERE content_hash = $1")
                    .bind(&hash)
                    .fetch_one(&self.pool)
                    .await?;

            return Ok(SaveReport {
                record: WorkspaceRecord {
                    id,
                    workspace,
                    creation_time,
                },
                is_new: false,
            });
        };
        let (paths, contents): (Vec<&str>, Vec<&str>) = workspace
            .files
            .iter()
            .map(|(path, content)| (path.as_str(), content.as_str()))
            .unzip();

        sqlx::query(
            "INSERT INTO workspace_file (workspace_id, path, content) \
             SELECT $1, * FROM UNNEST($2::text[], $3::text[])",
        )
        .bind(id)
        .bind(&paths)
        .bind(&contents)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        Ok(SaveReport {
            record: WorkspaceRecord {
                id,
                workspace,
                creation_time,
            },
            is_new: true,
        })
    }

    async fn find(&self, id: Uuid) -> Result<Option<WorkspaceRecord>, StoreError> {
        match self.find_workspace(id).await? {
            Some(record) => Ok(Some(record)),
            None => self.find_legacy(id).await,
        }
    }

    async fn check(&self) -> Result<(), StoreError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;

        Ok(())
    }
}
