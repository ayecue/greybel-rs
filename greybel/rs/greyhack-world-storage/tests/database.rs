use greyhack_world_storage::sql::{execute, fetch_column, fetch_value};
use greyhack_world_storage::{Database, DomainSchema, Schema};

use crate::support::database::{
    CHILDREN, PARENTS, VALUE_MIGRATIONS, VALUES_V1, VALUES_V3, database_path, ids, schema,
};

#[tokio::test]
async fn should_create_the_registered_tables() {
    // Arrange
    let database = Database::in_memory(&schema()).await.unwrap();

    // Act
    let names = fetch_column::<String>(
        database.connection(),
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        (),
    )
    .await
    .unwrap();

    // Assert
    assert_eq!(
        names,
        ["children", "parents", "schema_versions", "sessions"]
    );
}

#[tokio::test]
async fn should_clear_volatile_tables_when_a_database_opens() {
    // Arrange
    let directory = std::env::temp_dir().join(format!("world-storage-{}", std::process::id()));
    let path = directory.with_extension("db");
    let path = path.to_str().unwrap();
    let first = Database::open(path, &schema()).await.unwrap();

    execute(
        first.connection(),
        "INSERT INTO parents VALUES (1, 'kept')",
        (),
    )
    .await
    .unwrap();
    execute(first.connection(), "INSERT INTO sessions VALUES (1)", ())
        .await
        .unwrap();
    drop(first);

    // Act
    let second = Database::open(path, &schema()).await.unwrap();

    // Assert
    assert_eq!(ids(&second, "parents").await, [1]);
    assert_eq!(ids(&second, "sessions").await, Vec::<i64>::new());

    drop(second);

    for extension in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{path}{extension}"));
    }
}

#[tokio::test]
async fn should_commit_and_roll_back_the_open_transaction() {
    // Arrange
    let database = Database::in_memory(&schema()).await.unwrap();

    // Act
    database.begin().await.unwrap();

    let open = database.in_transaction().unwrap();

    execute(
        database.connection(),
        "INSERT INTO parents VALUES (1, 'a')",
        (),
    )
    .await
    .unwrap();
    database.rollback().await.unwrap();

    database.begin().await.unwrap();
    execute(
        database.connection(),
        "INSERT INTO parents VALUES (2, 'b')",
        (),
    )
    .await
    .unwrap();
    database.commit().await.unwrap();

    // Assert
    assert!(open);
    assert!(!database.in_transaction().unwrap());
    assert_eq!(ids(&database, "parents").await, [2]);
}

#[tokio::test]
async fn should_copy_every_registered_table_into_another_database() {
    // Arrange
    let source = Database::in_memory(&schema()).await.unwrap();
    let target = Database::in_memory(&schema()).await.unwrap();

    execute(
        source.connection(),
        "INSERT INTO parents VALUES (1, 'a')",
        (),
    )
    .await
    .unwrap();
    execute(
        source.connection(),
        "INSERT INTO children VALUES (5, 1)",
        (),
    )
    .await
    .unwrap();
    execute(
        target.connection(),
        "INSERT INTO parents VALUES (9, 'old')",
        (),
    )
    .await
    .unwrap();

    // Act
    source.copy_into(&target).await.unwrap();

    // Assert
    assert_eq!(ids(&target, "parents").await, [1]);
    assert_eq!(ids(&target, "children").await, [5]);
    assert!(!target.in_transaction().unwrap());
}

#[tokio::test]
async fn should_create_domain_tables_and_version_rows_on_a_fresh_database() {
    let schema = Schema::new().with_domain(DomainSchema::new("test", [PARENTS, CHILDREN]));
    let database = Database::in_memory(&schema).await.unwrap();

    assert_eq!(ids(&database, "parents").await, Vec::<i64>::new());
    assert_eq!(
        fetch_value::<i64>(
            database.connection(),
            "SELECT version FROM schema_versions WHERE domain = 'test'",
            (),
        )
        .await
        .unwrap(),
        Some(1)
    );
}

#[tokio::test]
async fn should_run_sql_and_rust_migrations_in_order() {
    let path = database_path("migrations");
    let v1 = Schema::new().with_domain(DomainSchema::new("test", [VALUES_V1]));
    let database = Database::open(&path, &v1).await.unwrap();

    execute(
        database.connection(),
        "INSERT INTO test_values VALUES (1, 3)",
        (),
    )
    .await
    .unwrap();
    drop(database);

    let v3 = Schema::new()
        .with_domain(DomainSchema::new("test", [VALUES_V3]).with_migrations(3, VALUE_MIGRATIONS));
    let database = Database::open(&path, &v3).await.unwrap();

    assert_eq!(
        fetch_value::<i64>(
            database.connection(),
            "SELECT doubled FROM test_values WHERE id = 1",
            ()
        )
        .await
        .unwrap(),
        Some(6)
    );
    assert_eq!(
        fetch_value::<i64>(
            database.connection(),
            "SELECT version FROM schema_versions WHERE domain = 'test'",
            (),
        )
        .await
        .unwrap(),
        Some(3)
    );
}

#[tokio::test]
async fn should_refuse_legacy_tables_without_a_domain_version() {
    let path = database_path("legacy");

    drop(Database::open(&path, &schema()).await.unwrap());

    let versioned = Schema::new().with_domain(DomainSchema::new("test", [PARENTS]));
    let error = Database::open(&path, &versioned).await.err().unwrap();

    assert!(error.to_string().contains("no schema version"));
}

#[tokio::test]
async fn should_refuse_a_newer_domain_version() {
    let path = database_path("newer-version");
    let schema = Schema::new().with_domain(DomainSchema::new("test", [PARENTS]));
    let database = Database::open(&path, &schema).await.unwrap();

    execute(
        database.connection(),
        "UPDATE schema_versions SET version = 99",
        (),
    )
    .await
    .unwrap();
    drop(database);

    let error = Database::open(&path, &schema).await.err().unwrap();

    assert!(error.to_string().contains("newer than supported"));
}

#[tokio::test]
async fn should_refuse_an_invalid_domain_version() {
    let path = database_path("invalid-version");
    let schema = Schema::new().with_domain(DomainSchema::new("test", [PARENTS]));
    let database = Database::open(&path, &schema).await.unwrap();

    execute(
        database.connection(),
        "UPDATE schema_versions SET version = 0",
        (),
    )
    .await
    .unwrap();
    drop(database);

    let error = Database::open(&path, &schema).await.err().unwrap();

    assert!(error.to_string().contains("invalid schema version"));
}
