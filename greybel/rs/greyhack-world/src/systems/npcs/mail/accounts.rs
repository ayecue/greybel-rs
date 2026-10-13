//! Mail accounts and the SMTP users of a mail server: creation, authentication, and restoring.

use greyhack_behaviour::{DEFAULT_MAIL_DOMAIN, ServiceId, is_lan_ip, is_valid_ip, verify_password};

use crate::model::{Email, MailAccount};
use crate::systems::computer::users::accounts::Login;
use crate::{ComputerId, Result, SmtpFailure, World};

impl World {
    /// Resolves SMTP for `smtp_user_list` (`PlayerUtils.GetSmtpServer`). LAN queries ignore the
    /// supplied port; public queries must forward to the actual SMTP port.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn smtp_users(
        &mut self,
        from: ComputerId,
        address: &str,
        port: u16,
    ) -> Result<Result<Vec<String>, SmtpFailure>> {
        if !is_valid_ip(address) {
            return Ok(Err(SmtpFailure::InvalidAddress));
        }

        if self.computer(from).await?.is_none() {
            return Ok(Err(SmtpFailure::HostDown));
        }

        let target = if is_lan_ip(address) {
            match self.router_of(from).await? {
                Some(router) => self.device(router.network, address).await?,
                None => None,
            }
        } else {
            let Some(router) = self.router_or_generate(address).await? else {
                return Ok(Err(SmtpFailure::UnknownAddress));
            };
            let table = self.port_table(router).await?;
            let Some(entry) = table.iter().find(|entry| entry.number == port) else {
                return Ok(Err(SmtpFailure::MissingPort(port)));
            };
            let Some(network) = self.router(router).await?.map(|router| router.network) else {
                return Ok(Err(SmtpFailure::UnknownAddress));
            };
            let target = self.device(network, &entry.lan_ip).await?;
            let valid = match target {
                Some(target) => self
                    .db
                    .port(target, entry.port.number)
                    .await?
                    .is_some_and(|open| open.service == ServiceId::Smtp),
                None => false,
            };

            if !valid {
                return Ok(Err(SmtpFailure::InvalidService));
            }

            target
        };
        let Some(target) = target else {
            return Ok(Err(SmtpFailure::UnknownHost));
        };
        let has_smtp = self
            .ports(target)
            .await?
            .iter()
            .any(|open| open.service == ServiceId::Smtp);

        if !has_smtp {
            return Ok(Err(SmtpFailure::MissingService));
        }

        self.smtp_user_list(target).await.map(Ok)
    }

    /// Returns usernames on SMTP `computer` for `smtp_user_list` (`CryptoIntrinsics`
    /// `smtp_user_list`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn smtp_user_list(&self, computer: ComputerId) -> Result<Vec<String>> {
        Ok(self
            .users(computer)
            .await?
            .into_iter()
            .map(|user| user.name)
            .collect())
    }

    /// Returns the mail domain for `computer`: its router domain or [`DEFAULT_MAIL_DOMAIN`] without
    /// one.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub(crate) async fn mail_domain(&self, computer: ComputerId) -> Result<String> {
        Ok(self
            .router_of(computer)
            .await?
            .map(|router| router.domain)
            .filter(|domain| !domain.is_empty())
            .unwrap_or_else(|| DEFAULT_MAIL_DOMAIN.to_owned()))
    }

    /// Creates the player mail account with its welcome message.
    pub(crate) async fn add_player_mail(&mut self, address: &str, password: &str) -> Result<()> {
        self.db.insert_mail_account(address, password).await?;
        self.db
            .insert_email(
                address,
                "welcome@greyhack.com",
                "Welcome",
                "Welcome to the net. Keep your passwords safe.",
            )
            .await?;

        Ok(())
    }

    /// Returns mail account `address` and its inbox, oldest first.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn mail_account(&self, address: &str) -> Result<Option<MailAccount>> {
        let Some(mut account) = self.db.mail_account(address).await? else {
            return Ok(None);
        };

        account.inbox = self.db.inbox(address).await?;

        Ok(Some(account))
    }

    /// Creates mail account `address` with `password`. Returns false if it exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_mail_account(&mut self, address: &str, password: &str) -> Result<bool> {
        if self.db.mail_account(address).await?.is_some() {
            return Ok(false);
        }

        self.db.insert_mail_account(address, password).await?;

        Ok(true)
    }

    /// Validates `password` for mail `address` (`Database.GetMailAccount`). Wrong passwords are
    /// refusal; accounts owned by dead or fired NPCs are disabled. The first login to an NPC account
    /// generates its mails.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn mail_login(&mut self, address: &str, password: &str) -> Result<Login> {
        let valid = self
            .db
            .mail_account(address)
            .await?
            .is_some_and(|account| verify_password(&account.password_hash, password));

        if !valid {
            return Ok(Login::InvalidCredentials);
        }

        if self.is_mail_disabled(address).await? {
            return Ok(Login::AccountInactive);
        }

        self.generate_npc_mail(address).await?;

        Ok(Login::Valid)
    }

    /// Returns whether mail `address` belongs to a dead or fired NPC (`Database.GetMailAccount` with
    /// `requireActiveAccount`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn is_mail_account_disabled(&self, address: &str) -> Result<bool> {
        self.is_mail_disabled(address).await
    }

    /// Returns whether mail `address` belongs to a dead or fired NPC (`Database.GetMailAccount`,
    /// `requireActiveAccount`).
    pub(super) async fn is_mail_disabled(&self, address: &str) -> Result<bool> {
        let Some(person) = self.db.mail_owner(address).await? else {
            return Ok(false);
        };

        Ok(self
            .db
            .npc(person)
            .await?
            .is_some_and(|npc| npc.is_dead_or_fired(self.time_seconds)))
    }

    /// Replaces mail account `address` with a copy of a Grey Hack account: its `password`, its
    /// `emails` (newest first, as `UserMail.emails` lists them) and whether its NPC mails were
    /// generated (`UserMail.npcContentsGenerated`). Creates the account if needed.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn restore_mail_account(
        &mut self,
        address: &str,
        password: &str,
        generated: bool,
        emails: &[Email],
    ) -> Result<()> {
        self.db.insert_mail_account(address, password).await?;
        self.db.clear_inbox(address).await?;

        for email in emails.iter().rev() {
            let id = self
                .db
                .insert_email(address, &email.from, &email.subject, &email.body)
                .await?;

            for reply in &email.replies {
                self.db.insert_reply(address, &id, reply).await?;
            }
        }

        self.db.set_npc_contents(address, generated).await
    }
}
