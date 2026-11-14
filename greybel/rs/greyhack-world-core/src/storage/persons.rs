//! Base person identities. Mail and bank links are stored by their domains.

use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_opt, insert};
use crate::{Person, PersonId, Result};

impl FromRow for Person {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: PersonId(row.get("id")?),
            name: row.get("name")?,
            surname: row.get("surname")?,
            gender: row.get_code("gender")?,
        })
    }
}

/// Queries of the tables owned by this area.
pub(crate) struct PersonRepository {
    db: Database,
}

impl PersonRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Returns person `id`.
    pub async fn person(&self, id: PersonId) -> Result<Option<Person>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT id, name, surname, gender FROM persons WHERE id = ?1",
            params![id.0],
        )
        .await?)
    }

    /// Creates `person` and returns its ID.
    pub async fn insert_person(&self, person: &Person) -> Result<PersonId> {
        let id = insert(
            &self.db.connection(),
            "INSERT INTO persons (name, surname, gender) VALUES (?1, ?2, ?3)",
            params![
                person.name.as_str(),
                person.surname.as_str(),
                person.gender.code()
            ],
        )
        .await?;

        Ok(PersonId(u32::try_from(id).expect("person ids fit u32")))
    }

    /// Updates `person`'s base identity.
    pub async fn save_person(&self, person: &Person) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE persons SET name = ?2, surname = ?3, gender = ?4 WHERE id = ?1",
            params![
                person.id.0,
                person.name.as_str(),
                person.surname.as_str(),
                person.gender.code()
            ],
        )
        .await?;

        Ok(())
    }
}
