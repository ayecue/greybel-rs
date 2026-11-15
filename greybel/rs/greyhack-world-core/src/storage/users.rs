//! Computer users, groups, and the global plaintext-password registry.

use greyhack_behaviour::hash_password;
use greyhack_world_storage::sql::{
    FromRow, RowView, execute, fetch_all, fetch_column, fetch_opt, fetch_value,
};
use turso::params;

use super::Database;
use crate::{ComputerId, PersonId, Result, User};

const USER_COLUMNS: &str = "computer_users.id, computer_users.name, computer_users.password_hash, \
     COALESCE(passwords.plain, '') AS password, computer_users.deletable, computer_users.person_id \
     FROM computer_users LEFT JOIN passwords ON passwords.hash = computer_users.password_hash";

/// User row and ID referenced by group rows.
struct UserRow(i64, User);

impl FromRow for UserRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self(
            row.get("id")?,
            User {
                name: row.get("name")?,
                password_hash: row.get("password_hash")?,
                password: row.get("password")?,
                deletable: row.get("deletable")?,
                person: row.get::<Option<u32>>("person_id")?.map(PersonId),
                groups: Vec::new(),
            },
        ))
    }
}

/// Queries of the tables owned by this area.
pub(crate) struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Stores `plain` under its hash and returns that hash.
    pub async fn register_password(&self, plain: &str) -> Result<String> {
        let hash = hash_password(plain);

        self.register_hash(&hash, plain).await?;

        Ok(hash)
    }

    /// Stores `plain` under `hash` if the hash is not registered.
    pub async fn register_hash(&self, hash: &str, plain: &str) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR IGNORE INTO passwords (hash, plain) VALUES (?1, ?2)",
            params![hash, plain],
        )
        .await?;

        Ok(())
    }

    /// Returns plaintext registered under `hash` (`Database.Passwords`).
    pub async fn plain_password(&self, hash: &str) -> Result<Option<String>> {
        Ok(fetch_value(
            &self.db.connection(),
            "SELECT plain FROM passwords WHERE hash = ?1",
            params![hash],
        )
        .await?)
    }

    /// Returns `computer` users in creation order.
    pub async fn users(&self, computer: ComputerId) -> Result<Vec<User>> {
        let rows: Vec<UserRow> = fetch_all(
            &self.db.connection(),
            &format!("SELECT {USER_COLUMNS} WHERE computer_users.computer_id = ?1 ORDER BY computer_users.id"),
            params![computer.0],
        )
        .await?;
        let mut users = Vec::with_capacity(rows.len());

        for UserRow(id, mut user) in rows {
            user.groups = self.groups(id).await?;
            users.push(user);
        }

        Ok(users)
    }

    /// Returns user `name` of `computer`.
    pub async fn user(&self, computer: ComputerId, name: &str) -> Result<Option<User>> {
        let row: Option<UserRow> = fetch_opt(
            &self.db.connection(),
            &format!("SELECT {USER_COLUMNS} WHERE computer_users.computer_id = ?1 AND computer_users.name = ?2"),
            params![computer.0, name],
        )
        .await?;
        let Some(UserRow(id, mut user)) = row else {
            return Ok(None);
        };

        user.groups = self.groups(id).await?;

        Ok(Some(user))
    }

    /// Returns how many users `computer` has.
    pub async fn user_count(&self, computer: ComputerId) -> Result<usize> {
        let count: i64 = fetch_value(
            &self.db.connection(),
            "SELECT count(*) FROM computer_users WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?
        .unwrap_or(0);

        Ok(usize::try_from(count).unwrap_or(0))
    }

    /// Returns the first person with an account on `computer`.
    pub async fn person_on(&self, computer: ComputerId) -> Result<Option<PersonId>> {
        let person: Option<u32> = fetch_value(
            &self.db.connection(),
            "SELECT person_id FROM computer_users WHERE computer_id = ?1 AND person_id IS NOT NULL ORDER BY id \
             LIMIT 1",
            params![computer.0],
        )
        .await?;

        Ok(person.map(PersonId))
    }

    /// Adds `user` to `computer`.
    pub async fn insert_user(&self, computer: ComputerId, user: &User) -> Result<()> {
        self.register_hash(&user.password_hash, &user.password)
            .await?;
        execute(
            &self.db.connection(),
            "INSERT INTO computer_users (computer_id, name, password_hash, person_id, deletable) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                computer.0,
                user.name.as_str(),
                user.password_hash.as_str(),
                user.person.map(|person| person.0),
                user.deletable
            ],
        )
        .await?;

        for group in &user.groups {
            self.add_group(computer, &user.name, group).await?;
        }

        Ok(())
    }

    /// Updates password, owner, and deletability. Groups change through [`Self::add_group`] and
    /// [`Self::remove_group`].
    pub async fn save_user(&self, computer: ComputerId, user: &User) -> Result<()> {
        self.register_hash(&user.password_hash, &user.password)
            .await?;
        execute(
            &self.db.connection(),
            "UPDATE computer_users SET password_hash = ?3, person_id = ?4, deletable = ?5 \
             WHERE computer_id = ?1 AND name = ?2",
            params![
                computer.0,
                user.name.as_str(),
                user.password_hash.as_str(),
                user.person.map(|person| person.0),
                user.deletable
            ],
        )
        .await?;

        Ok(())
    }

    /// Removes user `name` from `computer`. Returns whether it existed.
    pub async fn remove_user(&self, computer: ComputerId, name: &str) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "DELETE FROM computer_users WHERE computer_id = ?1 AND name = ?2",
            params![computer.0, name],
        )
        .await?
            > 0)
    }

    async fn groups(&self, user: i64) -> Result<Vec<String>> {
        Ok(fetch_column(
            &self.db.connection(),
            "SELECT name FROM user_groups WHERE user_id = ?1 ORDER BY id",
            params![user],
        )
        .await?)
    }

    /// Adds `group` to `user` on `computer`. Returns whether the user exists and lacked the group.
    pub async fn add_group(&self, computer: ComputerId, user: &str, group: &str) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "INSERT OR IGNORE INTO user_groups (user_id, name) \
             SELECT id, ?3 FROM computer_users WHERE computer_id = ?1 AND name = ?2",
            params![computer.0, user, group],
        )
        .await?
            > 0)
    }

    /// Removes `user` from `group` on `computer`. Returns whether the user was in it.
    pub async fn remove_group(
        &self,
        computer: ComputerId,
        user: &str,
        group: &str,
    ) -> Result<bool> {
        Ok(execute(
            &self.db.connection(),
            "DELETE FROM user_groups WHERE name = ?3 AND user_id = \
             (SELECT id FROM computer_users WHERE computer_id = ?1 AND name = ?2)",
            params![computer.0, user, group],
        )
        .await?
            > 0)
    }
}
