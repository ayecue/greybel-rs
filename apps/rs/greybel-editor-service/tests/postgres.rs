//! Tests `PostgresStore` against the database at `TEST_DATABASE_URL`; skipped without it.

use std::collections::BTreeMap;

use greybel_editor_service::store::{PostgresStore, WorkspaceStore};
use greybel_editor_service::workspace::Workspace;
use uuid::Uuid;

use crate::support::run;

#[test]
fn should_store_workspaces_and_read_legacy_projects_in_postgres() {
    let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
        eprintln!("TEST_DATABASE_URL is not set; skipping the Postgres test");

        return;
    };

    run(async {
        let pool = sqlx::PgPool::connect(&url).await.expect("connects");
        let legacy_id = Uuid::new_v4();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS \"Code\" (\"id\" TEXT NOT NULL PRIMARY KEY, \"content\" TEXT NOT NULL, \
             \"createdAt\" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .expect("creates the legacy table");
        sqlx::query("INSERT INTO \"Code\" (\"id\", \"content\") VALUES ($1, 'print 1')")
            .bind(legacy_id.to_string())
            .execute(&pool)
            .await
            .expect("inserts");

        let store = PostgresStore::connect(&url, 2)
            .await
            .expect("connects and migrates");
        let workspace = Workspace {
            entry: "main.src".to_owned(),
            files: BTreeMap::from([
                (
                    "main.src".to_owned(),
                    format!("print \"{}\"", Uuid::new_v4()),
                ),
                ("lib/util.src".to_owned(), "x = 1".to_owned()),
            ]),
        };
        let saved = store.save(workspace.clone()).await.expect("saves");
        let again = store.save(workspace.clone()).await.expect("saves");
        let found = store
            .find(saved.record.id)
            .await
            .expect("reads")
            .expect("finds");
        let legacy = store.find(legacy_id).await.expect("reads").expect("finds");
        let unknown = store.find(Uuid::new_v4()).await.expect("reads");

        assert_eq!(again.record.id, saved.record.id);
        assert!(saved.is_new);
        assert!(!again.is_new);
        assert_eq!(found.workspace, workspace);
        assert_eq!(
            legacy.workspace.files,
            BTreeMap::from([("main.src".to_owned(), "print 1".to_owned())])
        );
        assert_eq!(unknown, None);
        store.check().await.expect("answers");
    });
}
