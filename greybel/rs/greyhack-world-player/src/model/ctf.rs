//! CTF accounts, events, and participant completion. Accounts, creators, and players reference
//! persons and their mail and bank accounts.

use greyhack_world_core::PersonId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtfAccount {
    pub person: PersonId,
    pub user: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtfPlayer {
    pub event: String,
    pub person: PersonId,
    /// Player mail address and bank account loaded from the person.
    pub mail_user: String,
    pub bank_user: String,
    pub completed: bool,
    pub quality_rate: i64,
    pub difficulty_rate: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtfEvent {
    pub name: String,
    pub creator: PersonId,
    pub user: String,
    pub password_hash: String,
    pub description: String,
    pub public_address: String,
    pub creator_name: String,
    pub mail_content: String,
    /// Creator bank account loaded from the person.
    pub creator_bank_user: String,
    /// Rating sums and vote count, stored on the event (`CTF_Event`). They are 3, 3 and 1 until the first
    /// rating, which starts them over.
    pub quality_rank: i64,
    pub difficulty_rank: i64,
    pub num_votes: i64,
    pub is_rated: bool,
}

/// CTF rating errors (`PlayerIntrinsics` CTF rating).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtfFailure {
    /// Scores must be between 1 and 5.
    InvalidScore,
    /// No event has that name.
    UnknownEvent,
    /// The rater has not completed the event mission.
    Incomplete,
    /// The rater has already rated the event.
    RepeatScore,
    /// The rater created the event.
    OwnEvent,
}
