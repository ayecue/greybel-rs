use greyhack_world_storage::sql::{FromRow, RowView, insert};
use greyhack_world_storage::{Database, Schema, TableSet};
use turso::params;

pub const ITEMS: TableSet = TableSet::new(
    "CREATE TABLE IF NOT EXISTS items (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, kind INTEGER, weight REAL);\n",
    &["items"],
);

#[derive(Debug, PartialEq)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub kind: Option<u8>,
}

impl FromRow for Item {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            kind: row.get("kind")?,
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum Color {
    Red,
    Blue,
}

impl TryFrom<i64> for Color {
    type Error = String;

    fn try_from(code: i64) -> Result<Self, String> {
        match code {
            0 => Ok(Self::Red),
            1 => Ok(Self::Blue),
            other => Err(format!("{other} is no color")),
        }
    }
}

pub async fn database() -> Database {
    let database = Database::in_memory(&Schema::new().with(ITEMS))
        .await
        .unwrap();
    let conn = database.connection();

    insert(
        conn,
        "INSERT INTO items (name, kind, weight) VALUES (?1, ?2, ?3)",
        params!["a", 1, 0.5],
    )
    .await
    .unwrap();
    insert(
        conn,
        "INSERT INTO items (name, kind, weight) VALUES (?1, ?2, ?3)",
        params!["b", turso::Value::Null, 2],
    )
    .await
    .unwrap();

    database
}
