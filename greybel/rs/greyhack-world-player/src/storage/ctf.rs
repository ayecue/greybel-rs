//! CTF account, event, and player storage.

use greyhack_world_core::{Database, PersonId};
use greyhack_world_storage::sql::{FromRow, RowView, execute, fetch_all, fetch_opt};
use turso::params;

use crate::{CtfAccount, CtfEvent, CtfPlayer, Result};

impl FromRow for CtfAccount {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            person: PersonId(row.get("person_id")?),
            user: row.get("user")?,
            password_hash: row.get("password_hash")?,
        })
    }
}

impl FromRow for CtfEvent {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            name: row.get("name")?,
            user: row.get("user")?,
            password_hash: row.get("password_hash")?,
            description: row.get("description")?,
            public_address: row.get("public_address")?,
            creator_name: row.get("creator_name")?,
            mail_content: row.get("mail_content")?,
            creator: PersonId(row.get("creator_id")?),
            creator_bank_user: row.get("creator_bank_user")?,
            quality_rank: row.get("quality_rank")?,
            difficulty_rank: row.get("difficulty_rank")?,
            num_votes: row.get("num_votes")?,
            is_rated: row.get("is_rated")?,
        })
    }
}

impl FromRow for CtfPlayer {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        Ok(Self {
            event: row.get("event_name")?,
            person: PersonId(row.get("person_id")?),
            mail_user: row.get("mail_user")?,
            bank_user: row.get("bank_user")?,
            completed: row.get("completed")?,
            quality_rate: row.get("quality_rate")?,
            difficulty_rate: row.get("difficulty_rate")?,
        })
    }
}

/// CTF event row with owner bank account.
macro_rules! ctf_event_query {
    ($where:literal) => {
        concat!(
            "SELECT ctf_events.name, ctf_events.user, ctf_events.password_hash, ctf_events.description, \
             ctf_events.public_address, ctf_events.creator_name, ctf_events.mail_content, ctf_events.creator_id, \
             ctf_events.quality_rank, ctf_events.difficulty_rank, ctf_events.num_votes, ctf_events.is_rated, \
             COALESCE(person_banks.bank_number, '') AS creator_bank_user \
             FROM ctf_events LEFT JOIN person_banks ON person_banks.person_id = ctf_events.creator_id ",
            $where
        )
    };
}

/// CTF player row with mail and bank accounts from its computer.
macro_rules! ctf_player_query {
    ($where:literal) => {
        concat!(
            "SELECT ctf_players.event_name, ctf_players.person_id, ctf_players.completed, ctf_players.quality_rate, \
             ctf_players.difficulty_rate, COALESCE(person_mail.address, '') AS mail_user, \
             COALESCE(person_banks.bank_number, '') AS bank_user FROM ctf_players \
             LEFT JOIN person_mail ON person_mail.person_id = ctf_players.person_id \
             LEFT JOIN person_banks ON person_banks.person_id = ctf_players.person_id ",
            $where
        )
    };
}

/// Queries of [`Database`] for ctf.
/// Queries of the tables owned by this area.
pub(crate) struct CtfRepository {
    db: Database,
}

impl CtfRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    pub(crate) async fn ctf_account(&self, person: PersonId) -> Result<Option<CtfAccount>> {
        Ok(fetch_opt(
            &self.db.connection(),
            "SELECT person_id, user, password_hash FROM ctf_accounts WHERE person_id = ?1",
            params![person.0],
        )
        .await?)
    }

    pub(crate) async fn set_ctf_account(&self, account: &CtfAccount) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO ctf_accounts (person_id, user, password_hash) VALUES (?1, ?2, ?3) \
             ON CONFLICT (person_id) DO UPDATE SET user = ?2, password_hash = ?3",
            params![
                account.person.0,
                account.user.as_str(),
                account.password_hash.as_str()
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn ctf_event(&self, name: &str) -> Result<Option<CtfEvent>> {
        Ok(fetch_opt(
            &self.db.connection(),
            ctf_event_query!("WHERE ctf_events.name = ?1"),
            params![name],
        )
        .await?)
    }

    pub(crate) async fn ctf_events_of(&self, creator: PersonId) -> Result<Vec<CtfEvent>> {
        Ok(fetch_all(
            &self.db.connection(),
            ctf_event_query!("WHERE ctf_events.creator_id = ?1 ORDER BY ctf_events.name"),
            params![creator.0],
        )
        .await?)
    }

    pub(crate) async fn set_ctf_event(&self, event: &CtfEvent) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT INTO ctf_events (name, user, password_hash, description, public_address, creator_name, \
             mail_content, creator_id, quality_rank, difficulty_rank, num_votes, is_rated) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT (name) DO UPDATE SET \
             user = excluded.user, password_hash = excluded.password_hash, description = excluded.description, \
             public_address = excluded.public_address, creator_name = excluded.creator_name, \
             mail_content = excluded.mail_content, creator_id = excluded.creator_id, \
             quality_rank = excluded.quality_rank, difficulty_rank = excluded.difficulty_rank, \
             num_votes = excluded.num_votes, is_rated = excluded.is_rated",
            params![
                event.name.as_str(),
                event.user.as_str(),
                event.password_hash.as_str(),
                event.description.as_str(),
                event.public_address.as_str(),
                event.creator_name.as_str(),
                event.mail_content.as_str(),
                event.creator.0,
                event.quality_rank,
                event.difficulty_rank,
                event.num_votes,
                event.is_rated,
            ],
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn ctf_players(&self, event: &str) -> Result<Vec<CtfPlayer>> {
        Ok(fetch_all(
            &self.db.connection(),
            ctf_player_query!("WHERE ctf_players.event_name = ?1 ORDER BY ctf_players.person_id"),
            params![event],
        )
        .await?)
    }

    pub(crate) async fn ctf_player(
        &self,
        event: &str,
        person: PersonId,
    ) -> Result<Option<CtfPlayer>> {
        Ok(fetch_opt(
            &self.db.connection(),
            ctf_player_query!("WHERE ctf_players.event_name = ?1 AND ctf_players.person_id = ?2"),
            params![event, person.0],
        )
        .await?)
    }

    pub(crate) async fn set_ctf_player(&self, player: &CtfPlayer) -> Result<()> {
        execute(
            &self.db.connection(),
            "INSERT OR REPLACE INTO ctf_players (event_name, person_id, completed, quality_rate, difficulty_rate) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                player.event.as_str(),
                player.person.0,
                player.completed,
                player.quality_rate,
                player.difficulty_rate,
            ],
        )
        .await?;

        Ok(())
    }
}
