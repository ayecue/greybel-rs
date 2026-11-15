//! NPC mail script-object handles and typed object-table access.

use std::any::Any;

use greyhack_world_core::{Access, ComputerId, WorldError};
use greyhack_world_programs::protocol::ObjectId;
use greyhack_world_programs::{
    Handle, ObjectClass, ObjectIdentity, ObjectReach, ObjectTable as TreeObjects, ObjectTableHandle,
};

use crate::Email;
use crate::routes::Result;

/// Logged-in mail account accessed from `access`'s computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailRef {
    /// Computer and user credentials that opened the account.
    pub access: Access,
    /// Account address.
    pub address: String,
    /// Inbox snapshot from login; sending mail does not update it (`HelperMailUser.userMail`).
    pub emails: Vec<Email>,
}

/// Logged-in mail account. Mail identity excludes the inbox snapshot (`HelperScript.AddMetaMail`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaMailHandle(pub MailRef);

impl Handle<ComputerId> for MetaMailHandle {
    const CLASS: ObjectClass = ObjectClass::MetaMail;

    fn identity(&self) -> ObjectIdentity {
        ObjectIdentity::of(Self::CLASS, &(&self.0.access, &self.0.address))
    }

    fn same_identity(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<Self>().is_some_and(|other| {
            self.0.access == other.0.access && self.0.address == other.0.address
        })
    }

    fn reach(&self) -> Option<ObjectReach<ComputerId>> {
        Some(ObjectReach::Computer(self.0.access.computer))
    }
}

/// Typed NPC mail-object lookup and mutation on a tree's object table.
pub trait ObjectTableExt {
    /// Returns mail access.
    fn meta_mail(&self, id: ObjectId) -> Result<MailRef>;
    /// Returns mutable mail access for inbox updates.
    fn meta_mail_mut(&mut self, id: ObjectId) -> Result<MailRef>;
    /// Replaces the inbox after an update.
    fn set_meta_mail(&mut self, id: ObjectId, mail: MailRef) -> Result<()>;
}

impl ObjectTableExt for TreeObjects<ComputerId> {
    fn meta_mail(&self, id: ObjectId) -> Result<MailRef> {
        Ok(self.get::<MetaMailHandle, WorldError>(id)?.0.clone())
    }

    fn meta_mail_mut(&mut self, id: ObjectId) -> Result<MailRef> {
        Ok(self.get_mut::<MetaMailHandle, WorldError>(id)?.0.clone())
    }

    fn set_meta_mail(&mut self, id: ObjectId, mail: MailRef) -> Result<()> {
        self.get_mut::<MetaMailHandle, WorldError>(id)?.0 = mail;

        Ok(())
    }
}

impl ObjectTableExt for ObjectTableHandle<ComputerId> {
    fn meta_mail(&self, id: ObjectId) -> Result<MailRef> {
        self.read(|objects| ObjectTableExt::meta_mail(objects, id))
    }

    fn meta_mail_mut(&mut self, id: ObjectId) -> Result<MailRef> {
        self.write(|objects| ObjectTableExt::meta_mail_mut(objects, id))
    }

    fn set_meta_mail(&mut self, id: ObjectId, mail: MailRef) -> Result<()> {
        self.write(|objects| ObjectTableExt::set_meta_mail(objects, id, mail))
    }
}
