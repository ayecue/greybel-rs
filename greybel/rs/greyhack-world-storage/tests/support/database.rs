use greyhack_world_storage::sql::fetch_column;
use greyhack_world_storage::{
    Database, Migration, MigrationFuture, MigrationStep, Schema, TableSet,
};
use turso::Connection;

pub const PARENTS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS parents (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\n",
    &["parents"],
);
pub const CHILDREN: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS children (id INTEGER PRIMARY KEY, parent_id INTEGER NOT NULL REFERENCES parents(id));\n",
    &["children"],
);
pub const SESSIONS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS sessions (id INTEGER PRIMARY KEY);\n",
    &["sessions"],
)
.with_volatile("DELETE FROM sessions;\n");

pub fn schema() -> Schema {
    Schema::new().with(PARENTS).with(CHILDREN).with(SESSIONS)
}

pub async fn ids(database: &Database, table: &str) -> Vec<i64> {
    fetch_column::<i64>(
        database.connection(),
        &format!("SELECT id FROM {table} ORDER BY id"),
        (),
    )
    .await
    .unwrap()
}

pub const VALUES_V1: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS test_values (id INTEGER PRIMARY KEY, value INTEGER NOT NULL);\n",
    &["test_values"],
);
pub const VALUES_V3: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS test_values (id INTEGER PRIMARY KEY, value INTEGER NOT NULL, doubled INTEGER NOT NULL DEFAULT 0);\n",
    &["test_values"],
);

pub fn double_values(conn: &Connection) -> MigrationFuture<'_> {
    Box::pin(async {
        conn.execute_batch("UPDATE test_values SET doubled = value * 2;")
            .await
    })
}

pub const VALUE_MIGRATIONS: &[Migration] = &[
    Migration {
        description: "add the derived value column",
        step: MigrationStep::Sql(
            "ALTER TABLE test_values ADD COLUMN doubled INTEGER NOT NULL DEFAULT 0;",
        ),
    },
    Migration {
        description: "populate derived values in Rust",
        step: MigrationStep::Rust(double_values),
    },
];

pub fn database_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "world-storage-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
        ))
        .with_extension("db")
        .to_string_lossy()
        .into_owned()
}
