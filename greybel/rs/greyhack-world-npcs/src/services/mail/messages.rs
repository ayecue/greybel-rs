//! Reading, sending, and deleting mail.

use greyhack_behaviour::{Refusal, check_mail};
use greyhack_world_assets::GameTextTable;
use greyhack_world_core::Result;

use crate::{Email, Mail};

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

/// Methods of [`World`] for npcs mail messages.
impl Mail {
    pub async fn refresh_mail(&self, address: &str) -> Result<Result<Vec<Email>, MailRefusal>> {
        if self.rows.mail_account(address).await?.is_none() {
            return Ok(Err(MailRefusal::Missing));
        }

        if self.is_mail_disabled(address).await? {
            return Ok(Err(MailRefusal::AccountInactive));
        }

        Ok(Ok(self.rows.inbox(address).await?))
    }

    pub async fn deliver_mail(
        &self,
        to: &str,
        from: &str,
        subject: &str,
        body: &str,
    ) -> Result<Option<String>> {
        if self.rows.mail_account(to).await?.is_none() {
            return Ok(None);
        }

        // `PlayerUtils.SendMail` loads the recipient with `Database.GetMailAccount`, which
        // generates an NPC's mails first.
        self.generate_npc_mail(to).await?;

        Ok(Some(self.rows.insert_email(to, from, subject, body).await?))
    }

    pub async fn delete_mail(&self, address: &str, id: &str) -> Result<bool> {
        self.rows.remove_email(address, id).await
    }

    pub async fn delete_mails(&self, address: &str, ids: &[String]) -> Result<bool> {
        let mut deleted = false;

        for id in ids {
            if self.delete_mail(address, id).await? {
                deleted = true;
            }
        }

        Ok(deleted)
    }

    pub async fn clear_inbox(&self, address: &str) -> Result<()> {
        self.rows.clear_inbox(address).await
    }

    pub async fn send_mail(
        &self,
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

    pub async fn add_conversation_mail(
        &self,
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
                        self.rows
                            .insert_email(address, speaker, "RE: stuff", &body)
                            .await?,
                    );
                }
                Some(id) => {
                    self.rows.insert_reply(address, id, &body).await?;
                }
            }
        }

        Ok(())
    }
}
