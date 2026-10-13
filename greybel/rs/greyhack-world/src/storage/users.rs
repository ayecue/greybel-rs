//! Computer users, groups, and the global plaintext-password registry.

use greyhack_behaviour::hash_password;
use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_column, fetch_opt, fetch_value};
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

impl Database {
    /// Stores `plain` under its hash and returns that hash.
    pub(crate) async fn register_password(&self, plain: &str) -> Result<String> {
        let hash = hash_password(plain);

        self.register_hash(&hash, plain).await?;

        Ok(hash)
    }

    /// Stores `plain` under `hash` if the hash is not registered.
    pub(crate) async fn register_hash(&self, hash: &str, plain: &str) -> Result<()> {
        execute(
            &self.conn,
            "INSERT OR IGNORE INTO passwords (hash, plain) VALUES (?1, ?2)",
            params![hash, plain],
        )
        .await?;

        Ok(())
    }

    /// Returns plaintext registered under `hash` (`Database.Passwords`).
    pub(crate) async fn plain_password(&self, hash: &str) -> Result<Option<String>> {
        Ok(fetch_value(
            &self.conn,
            "SELECT plain FROM passwords WHERE hash = ?1",
            params![hash],
        )
        .await?)
    }

    pub(crate) async fn users(&self, computer: ComputerId) -> Result<Vec<User>> {
        let rows: Vec<UserRow> = fetch_all(
            &self.conn,
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

    pub(crate) async fn user(&self, computer: ComputerId, name: &str) -> Result<Option<User>> {
        let row: Option<UserRow> = fetch_opt(
            &self.conn,
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

    pub(crate) async fn user_count(&self, computer: ComputerId) -> Result<usize> {
        let count: i64 = fetch_value(
            &self.conn,
            "SELECT count(*) FROM computer_users WHERE computer_id = ?1",
            params![computer.0],
        )
        .await?
        .unwrap_or(0);

        Ok(usize::try_from(count).unwrap_or(0))
    }

    /// Returns the first person with an account on `computer`.
    pub(crate) async fn person_on(&self, computer: ComputerId) -> Result<Option<PersonId>> {
        let person: Option<u32> = fetch_value(
            &self.conn,
            "SELECT person_id FROM computer_users WHERE computer_id = ?1 AND person_id IS NOT NULL ORDER BY id \
             LIMIT 1",
            params![computer.0],
        )
        .await?;

        Ok(person.map(PersonId))
    }

    pub(crate) async fn insert_user(&self, computer: ComputerId, user: &User) -> Result<()> {
        self.register_hash(&user.password_hash, &user.password)
            .await?;
        execute(
            &self.conn,
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
    pub(crate) async fn save_user(&self, computer: ComputerId, user: &User) -> Result<()> {
        self.register_hash(&user.password_hash, &user.password)
            .await?;
        execute(
            &self.conn,
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

    pub(crate) async fn remove_user(&self, computer: ComputerId, name: &str) -> Result<bool> {
        Ok(execute(
            &self.conn,
            "DELETE FROM computer_users WHERE computer_id = ?1 AND name = ?2",
            params![computer.0, name],
        )
        .await?
            > 0)
    }

    async fn groups(&self, user: i64) -> Result<Vec<String>> {
        Ok(fetch_column(
            &self.conn,
            "SELECT name FROM user_groups WHERE user_id = ?1 ORDER BY id",
            params![user],
        )
        .await?)
    }

    /// Adds `group` to `user` on `computer`. Returns whether the user exists and lacked the group.
    pub(crate) async fn add_group(
        &self,
        computer: ComputerId,
        user: &str,
        group: &str,
    ) -> Result<bool> {
        Ok(execute(
            &self.conn,
            "INSERT OR IGNORE INTO user_groups (user_id, name) \
             SELECT id, ?3 FROM computer_users WHERE computer_id = ?1 AND name = ?2",
            params![computer.0, user, group],
        )
        .await?
            > 0)
    }

    pub(crate) async fn remove_group(
        &self,
        computer: ComputerId,
        user: &str,
        group: &str,
    ) -> Result<bool> {
        Ok(execute(
            &self.conn,
            "DELETE FROM user_groups WHERE name = ?3 AND user_id = \
             (SELECT id FROM computer_users WHERE computer_id = ?1 AND name = ?2)",
            params![computer.0, user, group],
        )
        .await?
            > 0)
    }
}
