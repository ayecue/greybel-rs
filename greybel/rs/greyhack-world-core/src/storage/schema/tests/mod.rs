use std::collections::BTreeSet;

use greyhack_world_storage::sql::fetch_column;
use support::schema;

use crate::Database;

#[path = "support/mod.rs"]
mod support;

#[tokio::test]
async fn should_create_exactly_the_registered_tables() {
    // Arrange
    let schema = schema();
    let database = Database::in_memory_with(&schema).await.unwrap();

    // Act
    let created = fetch_column::<String>(
        &database.connection(),
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
         AND name NOT LIKE '__turso_internal%'",
        (),
    )
    .await
    .unwrap();

    // Assert
    let registered = schema.tables();

    assert_eq!(
        created.into_iter().collect::<BTreeSet<_>>(),
        registered.iter().map(|&table| table.to_owned()).collect()
    );
    assert_eq!(
        registered.len(),
        registered.iter().collect::<BTreeSet<_>>().len()
    );
}

#[test]
fn should_register_the_accounts_before_the_computers_that_reference_them() {
    // Arrange
    let tables = schema().tables();
    let position = |table: &str| tables.iter().position(|&name| name == table).unwrap();

    // Act
    let persons = position("persons");
    let computers = position("computers");
    let nodes = position("nodes");

    // Assert
    assert_eq!(position("schema_versions"), 0);
    assert_eq!(position("passwords"), 1);
    assert!(persons < computers);
    assert!(computers < position("computer_users"));
    assert!(computers < nodes);
    assert!(position("libraries") < position("node_libraries"));
}

#[test]
fn should_clear_no_rows_when_a_database_opens() {
    // Arrange
    let schema = schema();

    // Act
    let volatile = schema.volatile_sql();

    // Assert
    assert!(volatile.is_empty());
}
