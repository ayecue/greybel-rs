use std::collections::BTreeSet;

#[path = "support/mod.rs"]
mod support;

use greyhack_world_core::Database;
use greyhack_world_economy::{CoinRound, DailyUpkeep, Payment};
use greyhack_world_libraries::LibraryCheck;
use greyhack_world_npcs::NpcSwitch;
use greyhack_world_routines::{QUEUE_TABLES, Routine as _};
use greyhack_world_services::ServiceRestart;
use greyhack_world_storage::sql::fetch_column;
use greyhack_world_traces::{ActiveTrace, PassiveTrace};

use super::world_schema;
use crate::DatabaseOpen;

#[tokio::test]
async fn should_create_exactly_the_registered_tables() {
    // Arrange
    let database = Database::in_memory().await.unwrap();

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
    let registered = world_schema().tables();

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
fn should_register_the_tables_of_every_routine_kind_after_the_queue() {
    // Arrange
    let schema = world_schema();
    let tables = schema.tables();
    let position = |table: &str| tables.iter().position(|&name| name == table).unwrap();

    // Act
    let queue = position(QUEUE_TABLES.tables()[0]);
    let kinds = [
        ServiceRestart::tables(),
        CoinRound::tables(),
        NpcSwitch::tables(),
        ActiveTrace::tables(),
        PassiveTrace::tables(),
        Payment::tables(),
        LibraryCheck::tables(),
        DailyUpkeep::tables(),
    ];

    // Assert
    assert_eq!(kinds.len(), 8);

    for set in kinds {
        assert!(schema.sets().contains(&set));

        for table in set.tables() {
            assert!(position(table) > queue);
        }
    }
}

#[test]
fn should_clear_the_rows_that_do_not_outlive_the_program_when_a_database_opens() {
    // Arrange
    let schema = world_schema();

    // Act
    let volatile = schema.volatile_sql();

    // Assert
    for table in [
        "mining_subscribers",
        "sniffer_subscribers",
        "flood_subscribers",
        "active_traces",
        "library_weakness_uses",
        "library_reporters",
        "computer_processes",
        "computer_sessions",
        "routines",
    ] {
        assert!(volatile.contains(&format!("DELETE FROM {table}")));
    }
}

#[test]
fn should_match_the_baseline_schema_except_for_the_domain_state_and_account_link_moves() {
    // Arrange
    fn statements(sql: &str) -> BTreeSet<String> {
        sql.split(';')
            .filter_map(|statement| {
                let start = statement.find("CREATE ").filter(|_| {
                    statement.contains("TABLE IF NOT EXISTS")
                        || statement.contains("INDEX IF NOT EXISTS")
                })?;
                let mut statement = statement[start..]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");

                statement.push(';');

                Some(statement)
            })
            .collect()
    }

    let actual = statements(&world_schema().sql());
    let schema_versions = actual
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS schema_versions "))
        .unwrap()
        .clone();
    let world_meta = actual
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS world_meta "))
        .unwrap()
        .clone();
    let traffic_state = actual
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS traffic_state "))
        .unwrap()
        .clone();
    let persons = actual
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS persons "))
        .unwrap()
        .clone();
    let person_banks = actual
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS person_banks "))
        .unwrap()
        .clone();
    let person_mail = actual
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS person_mail "))
        .unwrap()
        .clone();
    let mut expected: BTreeSet<String> = support::BASELINE
        .iter()
        .map(|statement| statement.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();

    expected.insert(schema_versions);
    expected.insert(world_meta);
    expected.insert(traffic_state);

    let old_persons = expected
        .iter()
        .find(|statement| statement.starts_with("CREATE TABLE IF NOT EXISTS persons "))
        .unwrap()
        .clone();

    expected.remove(&old_persons);
    expected.insert(persons.clone());
    expected.insert(person_banks);
    expected.insert(person_mail);

    // Act
    let expected_tables = expected
        .iter()
        .filter_map(|statement| {
            statement
                .strip_prefix("CREATE TABLE IF NOT EXISTS ")
                .map(|tail| tail.split(' ').next().unwrap().to_owned())
        })
        .collect::<BTreeSet<_>>();
    let actual_tables = world_schema().tables().into_iter().map(str::to_owned);

    // Assert
    assert_eq!(actual, expected);
    assert_eq!(actual_tables.collect::<BTreeSet<_>>(), expected_tables);
    assert!(!persons.contains("REFERENCES"));
    assert!(!actual.contains(&old_persons));
    assert!(
        !actual.contains(
            &support::WORLD_META
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        )
    );
}
