//! Reading, sending, and deleting mail.

use greyhack_behaviour::{Refusal, check_mail};
use greyhack_world_assets::GameTextTable;

use crate::model::Email;
use crate::{Result, World};

/// Why a script mail was not sent (`MetaMailIntrinsics.send`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendMailFailure {
    /// The sender has no address.
    NoAccount,
    /// An argument breaks a rule.
    Refusal(Refusal),
    /// The sender's account belongs to a dead or fired NPC.
    AccountInactive,
    /// The recipient account does not exist.
    Undeliverable,
}

/// Mail account refresh errors (`HelperScript.HelperMailUser.RefreshAccount`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailRefusal {
    /// The account address no longer exists.
    Missing,
    /// The account belongs to a dead or fired NPC.
    AccountInactive,
}

impl World {
    /// Reloads mail `address` as `MetaMail` account refresh does
    /// (`HelperScript.HelperMailUser.RefreshAccount`). Returns `None` for missing or disabled
    /// accounts.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn refresh_mail(&self, address: &str) -> Result<Result<Vec<Email>, MailRefusal>> {
        if self.db.mail_account(address).await?.is_none() {
            return Ok(Err(MailRefusal::Missing));
        }

        if self.is_mail_disabled(address).await? {
            return Ok(Err(MailRefusal::AccountInactive));
        }

        Ok(Ok(self.db.inbox(address).await?))
    }

    /// Delivers email to `to` and returns its ID, or `None` if the account does not exist.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn deliver_mail(
        &mut self,
        to: &str,
        from: &str,
        subject: &str,
        body: &str,
    ) -> Result<Option<String>> {
        if self.db.mail_account(to).await?.is_none() {
            return Ok(None);
        }

        // `PlayerUtils.SendMail` loads the recipient with `Database.GetMailAccount`, which
        // generates an NPC's mails first.
        self.generate_npc_mail(to).await?;

        Ok(Some(self.db.insert_email(to, from, subject, body).await?))
    }

    /// Deletes email `id` from `address` and returns whether it existed.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn delete_mail(&mut self, address: &str, id: &str) -> Result<bool> {
        self.db.remove_email(address, id).await
    }

    /// Deletes emails `ids` from `address` and returns whether any existed.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn delete_mails(&mut self, address: &str, ids: &[String]) -> Result<bool> {
        let mut deleted = false;

        for id in ids {
            if self.delete_mail(address, id).await? {
                deleted = true;
            }
        }

        Ok(deleted)
    }

    /// Deletes every message in `address`'s inbox (`MetaMailIntrinsics.delete_all`).
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn clear_inbox(&mut self, address: &str) -> Result<()> {
        self.db.clear_inbox(address).await
    }

    /// Sends mail from `from` to `to` (`MetaMailIntrinsics.send`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn send_mail(
        &mut self,
        from: &str,
        to: &str,
        subject: &str,
        message: &str,
    ) -> Result<std::result::Result<(), SendMailFailure>> {
        if from.is_empty() {
            return Ok(Err(SendMailFailure::NoAccount));
        }

        if let Err(refusal) = check_mail(to, subject, message) {
            return Ok(Err(SendMailFailure::Refusal(refusal)));
        }

        // `PlayerUtils.SendMail` loads the sender with `requireActiveAccount`.
        if self.is_mail_account_disabled(from).await? {
            return Ok(Err(SendMailFailure::AccountInactive));
        }

        Ok(
            if self
                .deliver_mail(to, from, subject, message)
                .await?
                .is_some()
            {
                Ok(())
            } else {
                Err(SendMailFailure::Undeliverable)
            },
        )
    }

    /// Puts the conversation `conversation` (a game text of lines) with `receive_user` into the
    /// mailbox `address` (`ItemsWorldGen.AddNpcConvMail`): one mail whose messages are the lines,
    /// those spoken by the owner marked as sent.
    pub(crate) async fn add_conversation_mail(
        &mut self,
        address: &str,
        conversation: &str,
        receive_user: &str,
    ) -> Result<()> {
        #[derive(serde::Deserialize)]
        struct Line {
            #[serde(rename = "idNpc")]
            speaker: u8,
            text: String,
        }

        let Some(lines) = GameTextTable::english()
            .get(conversation)
            .and_then(|text| serde_json::from_str::<Vec<Line>>(text).ok())
        else {
            return Ok(());
        };
        let mut id = None;

        for line in lines {
            let speaker = match line.speaker {
                0 => address,
                1 => receive_user,
                _ => return Ok(()),
            };
            // `HelperConversation.ResolveText` clears the zero-day address of a conversation
            // without one.
            let message = line.text.replace("$IP_ZERODAY", "");
            let body = if speaker == address {
                format!("> {message}")
            } else {
                message
            };

            match &id {
                None => {
                    id = Some(
                        self.db
                            .insert_email(address, speaker, "RE: stuff", &body)
                            .await?,
                    );
                }
                Some(id) => {
                    self.db.insert_reply(address, id, &body).await?;
                }
            }
        }

        Ok(())
    }
}
