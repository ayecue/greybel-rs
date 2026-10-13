//! Person, mail account, and email storage.

use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert};
use crate::{Email, MailAccount, Person, PersonId, Result};

impl FromRow for Person {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: PersonId(row.get("id")?),
            name: row.get("name")?,
            surname: row.get("surname")?,
            gender: row.get("gender")?,
            mail_address: row.get("mail_address")?,
            bank_number: row.get("bank_number")?,
        })
    }
}

impl FromRow for Email {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            id: email_id(row.get("id")?),
            from: row.get("sender")?,
            subject: row.get("subject")?,
            body: row.get("body")?,
            replies: Vec::new(),
        })
    }
}

/// Message after the first of a mail thread (`UserMail.Mail.messages`).
struct ReplyRow {
    mail: String,
    body: String,
}

impl FromRow for ReplyRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            mail: email_id(row.get("email_id")?),
            body: row.get("body")?,
        })
    }
}

/// Formats an email ID as row ID minus one in eight hexadecimal digits; the first email is
/// `00000000`.
fn email_id(row: i64) -> String {
    format!("{:08X}", row - 1)
}

/// Converts displayed email `id` to its row ID.
fn email_row(id: &str) -> Option<i64> {
    i64::from_str_radix(id, 16).ok().map(|id| id + 1)
}

struct MailAccountRow {
    address: String,
    password_hash: String,
}

impl FromRow for MailAccountRow {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            address: row.get("address")?,
            password_hash: row.get("password_hash")?,
        })
    }
}

impl Database {
    pub(crate) async fn person(&self, id: PersonId) -> Result<Option<Person>> {
        Ok(fetch_opt(
            &self.conn,
            "SELECT id, name, surname, gender, mail_address, bank_number FROM persons WHERE id = ?1",
            params![id.0],
        )
        .await?)
    }

    /// Returns the person owning mail account `address`.
    pub(crate) async fn mail_owner(&self, address: &str) -> Result<Option<PersonId>> {
        let id: Option<u32> = fetch_value(
            &self.conn,
            "SELECT id FROM persons WHERE mail_address = ?1",
            params![address],
        )
        .await?;

        Ok(id.map(PersonId))
    }

    /// Deletes every email in `address`'s inbox.
    pub(crate) async fn clear_inbox(&self, address: &str) -> Result<()> {
        execute(
            &self.conn,
            "DELETE FROM emails WHERE address = ?1",
            params![address],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_person(&self, person: &Person) -> Result<PersonId> {
        let id = insert(
            &self.conn,
            "INSERT INTO persons (name, surname, gender, mail_address, bank_number) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                person.name.as_str(),
                person.surname.as_str(),
                person.gender.code(),
                person.mail_address.as_deref(),
                person.bank_number.as_deref()
            ],
        )
        .await?;

        Ok(PersonId(u32::try_from(id).expect("person ids fit u32")))
    }

    /// Updates `person` name and accounts.
    pub(crate) async fn save_person(&self, person: &Person) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE persons SET name = ?2, surname = ?3, gender = ?4, mail_address = ?5, bank_number = ?6 \
             WHERE id = ?1",
            params![
                person.id.0,
                person.name.as_str(),
                person.surname.as_str(),
                person.gender.code(),
                person.mail_address.as_deref(),
                person.bank_number.as_deref()
            ],
        )
        .await?;

        Ok(())
    }

    /// Returns mail account `address`.
    pub(crate) async fn mail_account(&self, address: &str) -> Result<Option<MailAccount>> {
        let account: Option<MailAccountRow> = fetch_opt(
            &self.conn,
            "SELECT address, password_hash FROM mail_accounts WHERE address = ?1",
            params![address],
        )
        .await?;

        Ok(account.map(|account| MailAccount {
            address: account.address,
            password_hash: account.password_hash,
            inbox: Vec::new(),
        }))
    }

    /// Creates `address` with `password`, or updates an existing password.
    pub(crate) async fn insert_mail_account(&self, address: &str, password: &str) -> Result<()> {
        let hash = self.register_password(password).await?;

        execute(
            &self.conn,
            "INSERT INTO mail_accounts (address, password_hash) VALUES (?1, ?2) \
             ON CONFLICT (address) DO UPDATE SET password_hash = excluded.password_hash",
            params![address, hash.as_str()],
        )
        .await?;

        Ok(())
    }

    /// Renames mail account `from` to `to`; its password, mails and owner stay with it. An account `to` that already
    /// exists is replaced.
    pub(crate) async fn rename_mail_account(&self, from: &str, to: &str) -> Result<()> {
        if from == to {
            return Ok(());
        }

        execute(
            &self.conn,
            "DELETE FROM mail_accounts WHERE address = ?1",
            params![to],
        )
        .await?;
        execute(
            &self.conn,
            "INSERT INTO mail_accounts (address, password_hash, npc_contents) \
             SELECT ?2, password_hash, npc_contents FROM mail_accounts WHERE address = ?1",
            params![from, to],
        )
        .await?;
        execute(
            &self.conn,
            "UPDATE emails SET address = ?2 WHERE address = ?1",
            params![from, to],
        )
        .await?;
        execute(
            &self.conn,
            "UPDATE persons SET mail_address = ?2 WHERE mail_address = ?1",
            params![from, to],
        )
        .await?;
        execute(
            &self.conn,
            "DELETE FROM mail_accounts WHERE address = ?1",
            params![from],
        )
        .await?;

        Ok(())
    }

    /// Returns `address` inbox emails newest first, as `UserMail.RecibirMail` inserts each new mail
    /// at the front of `UserMail.emails`.
    pub(crate) async fn inbox(&self, address: &str) -> Result<Vec<Email>> {
        let mut emails: Vec<Email> = fetch_all(
            &self.conn,
            "SELECT id, sender, subject, body FROM emails WHERE address = ?1 ORDER BY id DESC",
            params![address],
        )
        .await?;
        let replies: Vec<ReplyRow> = fetch_all(
            &self.conn,
            "SELECT email_replies.email_id, email_replies.body FROM email_replies \
             JOIN emails ON emails.id = email_replies.email_id WHERE emails.address = ?1 \
             ORDER BY email_replies.id",
            params![address],
        )
        .await?;

        for reply in replies {
            if let Some(email) = emails.iter_mut().find(|email| email.id == reply.mail) {
                email.replies.push(reply.body);
            }
        }

        Ok(emails)
    }

    /// Adds `body` as a further message of email `id` in `address` (`UserMail.RecibirMail` with the
    /// ID of an existing mail). Returns false if the email does not exist.
    pub(crate) async fn insert_reply(&self, address: &str, id: &str, body: &str) -> Result<bool> {
        let Some(row) = email_row(id) else {
            return Ok(false);
        };
        let exists: Option<i64> = fetch_value(
            &self.conn,
            "SELECT id FROM emails WHERE id = ?1 AND address = ?2",
            params![row, address],
        )
        .await?;

        if exists.is_none() {
            return Ok(false);
        }

        execute(
            &self.conn,
            "INSERT INTO email_replies (email_id, body) VALUES (?1, ?2)",
            params![row, body],
        )
        .await?;

        Ok(true)
    }

    /// Returns whether the NPC mails of `address` exist (`UserMail.npcContentsGenerated`).
    pub(crate) async fn has_npc_contents(&self, address: &str) -> Result<bool> {
        let present: Option<bool> = fetch_value(
            &self.conn,
            "SELECT npc_contents FROM mail_accounts WHERE address = ?1",
            params![address],
        )
        .await?;

        Ok(present.unwrap_or(false))
    }

    /// Records whether the NPC mails of `address` exist (`UserMail.npcContentsGenerated`).
    pub(crate) async fn set_npc_contents(&self, address: &str, present: bool) -> Result<()> {
        execute(
            &self.conn,
            "UPDATE mail_accounts SET npc_contents = ?2 WHERE address = ?1",
            params![address, present],
        )
        .await?;

        Ok(())
    }

    /// Delivers an email to `address` and returns its ID.
    pub(crate) async fn insert_email(
        &self,
        address: &str,
        from: &str,
        subject: &str,
        body: &str,
    ) -> Result<String> {
        let id = insert(
            &self.conn,
            "INSERT INTO emails (address, sender, subject, body) VALUES (?1, ?2, ?3, ?4)",
            params![address, from, subject, body],
        )
        .await?;

        Ok(email_id(id))
    }

    /// Deletes email `id` from `address` and returns whether it existed.
    pub(crate) async fn remove_email(&self, address: &str, id: &str) -> Result<bool> {
        let Some(row) = email_row(id) else {
            return Ok(false);
        };

        Ok(execute(
            &self.conn,
            "DELETE FROM emails WHERE id = ?1 AND address = ?2",
            params![row, address],
        )
        .await?
            > 0)
    }
}
