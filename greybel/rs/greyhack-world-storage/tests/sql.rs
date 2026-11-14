use greyhack_world_storage::sql::{
    Code, execute, fetch_all, fetch_column, fetch_opt, fetch_value, insert,
};
use turso::params;

use crate::support::sql::{Color, Item, database};

#[tokio::test]
async fn should_return_the_inserted_row_id() {
    // Arrange
    let database = database().await;

    // Act
    let id = insert(
        database.connection(),
        "INSERT INTO items (name) VALUES (?1)",
        params!["c"],
    )
    .await
    .unwrap();

    // Assert
    assert_eq!(id, 3);
}

#[tokio::test]
async fn should_map_rows_by_column_name() {
    // Arrange
    let database = database().await;

    // Act
    let items: Vec<Item> = fetch_all(
        database.connection(),
        "SELECT id, name, kind FROM items ORDER BY id",
        (),
    )
    .await
    .unwrap();

    // Assert
    assert_eq!(
        items,
        [
            Item {
                id: 1,
                name: "a".to_owned(),
                kind: Some(1)
            },
            Item {
                id: 2,
                name: "b".to_owned(),
                kind: None
            }
        ]
    );
}

#[tokio::test]
async fn should_map_the_first_row_if_any() {
    // Arrange
    let database = database().await;
    let conn = database.connection();

    // Act
    let found: Option<Item> = fetch_opt(
        conn,
        "SELECT id, name, kind FROM items WHERE id = ?1",
        params![2],
    )
    .await
    .unwrap();
    let missing: Option<Item> = fetch_opt(
        conn,
        "SELECT id, name, kind FROM items WHERE id = ?1",
        params![9],
    )
    .await
    .unwrap();

    // Assert
    assert_eq!(found.map(|item| item.name), Some("b".to_owned()));
    assert_eq!(missing, None);
}

#[tokio::test]
async fn should_read_columns_and_values() {
    // Arrange
    let database = database().await;
    let conn = database.connection();

    // Act
    let names = fetch_column::<String>(conn, "SELECT name FROM items ORDER BY id", ())
        .await
        .unwrap();
    let weight = fetch_value::<f64>(conn, "SELECT weight FROM items WHERE id = 2", ())
        .await
        .unwrap();
    let none = fetch_value::<i64>(conn, "SELECT id FROM items WHERE id = 9", ())
        .await
        .unwrap();
    let pairs = fetch_all::<(i64, String)>(conn, "SELECT id, name FROM items ORDER BY id", ())
        .await
        .unwrap();

    // Assert
    assert_eq!(names, ["a", "b"]);
    assert_eq!(weight, Some(2.0));
    assert_eq!(none, None);
    assert_eq!(pairs, [(1, "a".to_owned()), (2, "b".to_owned())]);
}

#[tokio::test]
async fn should_report_the_changed_row_count() {
    // Arrange
    let database = database().await;

    // Act
    let changed = execute(database.connection(), "DELETE FROM items", ())
        .await
        .unwrap();

    // Assert
    assert_eq!(changed, 2);
}

#[tokio::test]
async fn should_read_an_integer_code_as_its_type() {
    // Arrange
    let database = database().await;
    let conn = database.connection();

    // Act
    let color = fetch_value::<Code<Color>>(conn, "SELECT kind FROM items WHERE id = 1", ())
        .await
        .unwrap();
    let unknown = fetch_value::<Code<Color>>(conn, "SELECT 7", ()).await;

    // Assert
    assert_eq!(color, Some(Code(Color::Blue)));
    assert!(unknown.is_err());
}

#[tokio::test]
async fn should_reject_a_value_of_another_type() {
    // Arrange
    let database = database().await;

    // Act
    let result = fetch_value::<i64>(database.connection(), "SELECT name FROM items", ()).await;

    // Assert
    assert!(result.is_err());
}
