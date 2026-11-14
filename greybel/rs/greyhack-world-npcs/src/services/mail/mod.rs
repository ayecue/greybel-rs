//! Mail accounts, messages, and the mail the game generates for NPCs.

pub mod accounts;
pub mod messages;
pub mod npc_inbox;

use greyhack_world_core::{Dep, PersonId, Result, WorldRef};
pub use messages::{MailRefusal, SendMailFailure};

use crate::storage::{MailRepository, NetworkAdminRepository, NpcRepository};

/// Mail accounts, messages, NPC inboxes, and SMTP users.
pub struct Mail {
    pub(crate) world: WorldRef,
    pub(crate) mail: std::rc::Rc<MailRepository>,
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
            mail,
            npcs,
            network_admins,
            computers,
            networks,
            network_generation,
        }
    }

    /// Returns the mail address linked to `person`, if any.
    pub async fn mail_address(&self, person: PersonId) -> Result<Option<String>> {
        self.mail.mail_address(person).await
    }

    /// Links `person` to `address`, or removes that link when `address` is `None`.
    pub async fn set_mail_address(&self, person: PersonId, address: Option<&str>) -> Result<()> {
        self.mail.set_mail_address(person, address).await
    }

    /// Stores an NPC reply and reports whether its parent message accepted it.
    pub async fn insert_reply(&self, address: &str, id: &str, body: &str) -> Result<bool> {
        self.mail.insert_reply(address, id, body).await
    }

    /// Delivers a message to `address` and returns its ID.
    pub async fn insert_email(
        &self,
        address: &str,
        from: &str,
        subject: &str,
        body: &str,
    ) -> Result<String> {
        self.mail.insert_email(address, from, subject, body).await
    }
}
