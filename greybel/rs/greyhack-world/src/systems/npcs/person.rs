//! Persons: records and the account owner of a computer.

use crate::model::{ComputerId, Person, PersonId};
use crate::{Result, World};

impl World {
    /// Returns person `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn person(&self, id: PersonId) -> Result<Option<Person>> {
        self.db.person(id).await
    }

    /// Updates `person` name and accounts.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_person(&mut self, person: &Person) -> Result<()> {
        self.db.save_person(person).await
    }

    /// Returns the person owning an account on `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn person_on(&self, computer: ComputerId) -> Result<Option<PersonId>> {
        self.db.person_on(computer).await
    }
}
