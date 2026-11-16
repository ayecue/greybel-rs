//! Mail accounts, messages, and the mail the game generates for NPCs.

pub mod accounts;
pub mod messages;
pub mod npc_inbox;

use greyhack_world_core::{PersonId, Result, WorldRef};
use greyhack_world_modules::Dep;
pub use messages::{MailRefusal, SendMailFailure};

/// Spam-filter state carried by a mirrored `UserMail`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailSpamConfig {
    pub player_pc_id: String,
    pub filter: u8,
    pub blacklist: Vec<String>,
}

use crate::storage::{MailRepository, NetworkAdminRepository, NpcRepository};

/// Mail accounts, messages, NPC inboxes, and SMTP users.
pub struct Mail {
    pub(crate) world: WorldRef,
    pub(crate) rows: std::rc::Rc<MailRepository>,
    pub(crate) npcs: std::rc::Rc<NpcRepository>,
    pub(crate) network_admins: std::rc::Rc<NetworkAdminRepository>,
    pub(crate) computers: Dep<greyhack_world_core::ComputerService>,
    pub(crate) networks: Dep<greyhack_world_core::NetworkService>,
    pub(crate) network_generation: Dep<dyn greyhack_world_core::NetworkGeneration>,
}

impl Mail {
    pub(crate) fn new(
        world: &WorldRef,
        mail: std::rc::Rc<MailRepository>,
        npcs: std::rc::Rc<NpcRepository>,
        network_admins: std::rc::Rc<NetworkAdminRepository>,
        computers: Dep<greyhack_world_core::ComputerService>,
        networks: Dep<greyhack_world_core::NetworkService>,
        network_generation: Dep<dyn greyhack_world_core::NetworkGeneration>,
    ) -> Self {
        Self {
            world: world.clone(),
            rows: mail,
            npcs,
            network_admins,
            computers,
            networks,
            network_generation,
        }
    }

    /// Applies the spam-filter fields of a mirrored account and persists them with the world.
    pub async fn set_spam_config(&self, address: &str, config: MailSpamConfig) -> Result<()> {
        self.rows.set_spam_config(address, &config).await
    }

    pub(crate) async fn spam_config(&self, address: &str) -> Result<Option<MailSpamConfig>> {
        self.rows.spam_config(address).await
    }

    /// Returns the mail address linked to `person`, if any.
    pub async fn mail_address(&self, person: PersonId) -> Result<Option<String>> {
        self.rows.mail_address(person).await
    }

    /// Links `person` to `address`, or removes that link when `address` is `None`.
    pub async fn set_mail_address(&self, person: PersonId, address: Option<&str>) -> Result<()> {
        self.rows.set_mail_address(person, address).await
    }

    /// Stores an NPC reply and reports whether its parent message accepted it.
    pub async fn insert_reply(&self, address: &str, id: &str, body: &str) -> Result<bool> {
        self.rows.insert_reply(address, id, body).await
    }

    /// Delivers a message to `address` and returns its ID.
    pub async fn insert_email(
        &self,
        address: &str,
        from: &str,
        subject: &str,
        body: &str,
    ) -> Result<String> {
        self.rows.insert_email(address, from, subject, body).await
    }
}
