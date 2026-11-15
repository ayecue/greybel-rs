//! Mail account and email storage.

use greyhack_world_core::{Database, PersonId, Result};
use greyhack_world_storage::sql::{
    FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert,
};
use turso::params;

use crate::{Email, MailAccount};

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

/// Queries of [`Database`] for mail.
/// Queries of the tables owned by this area.
pub(crate) struct MailRepository {
    db: Database,
    computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
}

impl MailRepository {
    pub(crate) fn new(
        db: Database,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    ) -> Self {
        Self { db, computers }
    }

    pub(crate) async fn mail_owner(&self, address: &str) -> Result<Option<PersonId>> {
        let id: Option<u32> = fetch_value(
            &self.db.connection(),
            "SELECT person_id FROM person_mail WHERE address = ?1",
            params![address],
        )
        .await?;

        Ok(id.map(PersonId))
    }

    pub(crate) async fn mail_address(&self, person: PersonId) -> Result<Option<String>> {
        Ok(fetch_value(
            &self.db.connection(),
            "SELECT address FROM person_mail WHERE person_id = ?1",
            params![person.0],
        )
        .await?)
    }

    pub(crate) async fn set_mail_address(
        &self,
        person: PersonId,
        address: Option<&str>,
    ) -> Result<()> {
        match address {
            Some(address) => {
                execute(
                    &self.db.connection(),
                    "INSERT INTO person_mail (person_id, address) VALUES (?1, ?2) \
                     ON CONFLICT (person_id) DO UPDATE SET address = ?2",
                    params![person.0, address],
                )
                .await?
            }
            None => {
                execute(
                    &self.db.connection(),
                    "DELETE FROM person_mail WHERE person_id = ?1",
                    params![person.0],
                )
                .await?
            }
        };

        Ok(())
    }

    pub(crate) async fn clear_inbox(&self, address: &str) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM emails WHERE address = ?1",
            params![address],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn mail_account(&self, address: &str) -> Result<Option<MailAccount>> {
        let account: Option<MailAccountRow> = fetch_opt(
            &self.db.connection(),
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

    pub(crate) async fn insert_mail_account(&self, address: &str, password: &str) -> Result<()> {
        let hash = self.computers.register_password(password).await?;

        execute(
            &self.db.connection(),
            "INSERT INTO mail_accounts (address, password_hash) VALUES (?1, ?2) \
             ON CONFLICT (address) DO UPDATE SET password_hash = excluded.password_hash",
            params![address, hash.as_str()],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn rename_mail_account(&self, from: &str, to: &str) -> Result<()> {
        if from == to {
            return Ok(());
        }

        execute(
            &self.db.connection(),
            "DELETE FROM mail_accounts WHERE address = ?1",
            params![to],
        )
        .await?;
        execute(
            &self.db.connection(),
            "INSERT INTO mail_accounts (address, password_hash, npc_contents) \
             SELECT ?2, password_hash, npc_contents FROM mail_accounts WHERE address = ?1",
            params![from, to],
        )
        .await?;
        execute(
            &self.db.connection(),
            "UPDATE emails SET address = ?2 WHERE address = ?1",
            params![from, to],
        )
        .await?;
        execute(
            &self.db.connection(),
            "UPDATE person_mail SET address = ?2 WHERE address = ?1",
            params![from, to],
        )
        .await?;
        execute(
            &self.db.connection(),
            "DELETE FROM mail_accounts WHERE address = ?1",
            params![from],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn inbox(&self, address: &str) -> Result<Vec<Email>> {
        let mut emails: Vec<Email> = fetch_all(
            &self.db.connection(),
            "SELECT id, sender, subject, body FROM emails WHERE address = ?1 ORDER BY id DESC",
            params![address],
        )
        .await?;
        let replies: Vec<ReplyRow> = fetch_all(
            &self.db.connection(),
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

    pub(crate) async fn insert_reply(&self, address: &str, id: &str, body: &str) -> Result<bool> {
        let Some(row) = email_row(id) else {
            return Ok(false);
        };
        let exists: Option<i64> = fetch_value(
            &self.db.connection(),
            "SELECT id FROM emails WHERE id = ?1 AND address = ?2",
            params![row, address],
        )
        .await?;

        if exists.is_none() {
            return Ok(false);
        }

        execute(
            &self.db.connection(),
            "INSERT INTO email_replies (email_id, body) VALUES (?1, ?2)",
            params![row, body],
        )
        .await?;

        Ok(true)
    }

    pub(crate) async fn has_npc_contents(&self, address: &str) -> Result<bool> {
        let present: Option<bool> = fetch_value(
            &self.db.connection(),
            "SELECT npc_contents FROM mail_accounts WHERE address = ?1",
            params![address],
        )
        .await?;

        Ok(present.unwrap_or(false))
    }

    pub(crate) async fn set_npc_contents(&self, address: &str, present: bool) -> Result<()> {
        execute(
            &self.db.connection(),
            "UPDATE mail_accounts SET npc_contents = ?2 WHERE address = ?1",
            params![address, present],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn insert_email(
        &self,
        address: &str,
        from: &str,
        subject: &str,
        body: &str,
    ) -> Result<String> {
        let id = insert(
            &self.db.connection(),
            "INSERT INTO emails (address, sender, subject, body) VALUES (?1, ?2, ?3, ?4)",
            params![address, from, subject, body],
        )
        .await?;

        Ok(email_id(id))
    }

    pub(crate) async fn remove_email(&self, address: &str, id: &str) -> Result<bool> {
        let Some(row) = email_row(id) else {
            return Ok(false);
        };

        Ok(execute(
            &self.db.connection(),
            "DELETE FROM emails WHERE id = ?1 AND address = ?2",
            params![row, address],
        )
        .await?
            > 0)
    }
}
