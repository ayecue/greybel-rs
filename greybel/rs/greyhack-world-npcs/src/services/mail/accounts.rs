//! Mail accounts and the SMTP users of a mail server: creation, authentication, and restoring.

use greyhack_behaviour::{DEFAULT_MAIL_DOMAIN, ServiceId, is_lan_ip, is_valid_ip, verify_password};
use greyhack_world_core::{ComputerId, Login, Result};

use crate::{Email, Mail, MailAccount, SmtpFailure};

/// Methods of [`World`] for npcs mail accounts.
impl Mail {
    pub async fn smtp_users(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
    ) -> Result<Result<Vec<String>, SmtpFailure>> {
        if !is_valid_ip(address) {
            return Ok(Err(SmtpFailure::InvalidAddress));
        }

        if self.networks.topology.computer(from).await?.is_none() {
            return Ok(Err(SmtpFailure::HostDown));
        }

        let target = if is_lan_ip(address) {
            match self.networks.topology.router_of(from).await? {
                Some(router) => {
                    self.networks
                        .topology
                        .device(router.network, address)
                        .await?
                }
                None => None,
            }
        } else {
            let Some(router) = self
                .network_generation
                .router_or_generate(&self.world, address)
                .await?
            else {
                return Ok(Err(SmtpFailure::UnknownAddress));
            };
            let table = self.networks.lan.port_table(router).await?;
            let Some(entry) = table.iter().find(|entry| entry.number == port) else {
                return Ok(Err(SmtpFailure::MissingPort(port)));
            };
            let Some(network) = self
                .networks
                .routers
                .router(router)
                .await?
                .map(|router| router.network)
            else {
                return Ok(Err(SmtpFailure::UnknownAddress));
            };
            let target = self
                .networks
                .topology
                .device(network, &entry.lan_ip)
                .await?;
            let valid = match target {
                Some(target) => self
                    .computers
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
            .computers
            .processes
            .ports(target)
            .await?
            .iter()
            .any(|open| open.service == ServiceId::Smtp);

        if !has_smtp {
            return Ok(Err(SmtpFailure::MissingService));
        }

        self.smtp_user_list(target).await.map(Ok)
    }

    pub async fn smtp_user_list(&self, computer: ComputerId) -> Result<Vec<String>> {
        Ok(self
            .computers
            .users
            .users(computer)
            .await?
            .into_iter()
            .map(|user| user.name)
            .collect())
    }

    pub async fn mail_domain(&self, computer: ComputerId) -> Result<String> {
        Ok(self
            .networks
            .topology
            .router_of(computer)
            .await?
            .map(|router| router.domain)
            .filter(|domain| !domain.is_empty())
            .unwrap_or_else(|| DEFAULT_MAIL_DOMAIN.to_owned()))
    }

    pub async fn add_player_mail(&self, address: &str, password: &str) -> Result<()> {
        self.rows.insert_mail_account(address, password).await?;
        self.rows
            .insert_email(
                address,
                "welcome@greyhack.com",
                "Welcome",
                "Welcome to the net. Keep your passwords safe.",
            )
            .await?;

        Ok(())
    }

    pub async fn mail_account(&self, address: &str) -> Result<Option<MailAccount>> {
        let Some(mut account) = self.rows.mail_account(address).await? else {
            return Ok(None);
        };

        account.inbox = self.rows.inbox(address).await?;

        Ok(Some(account))
    }

    pub async fn add_mail_account(&self, address: &str, password: &str) -> Result<bool> {
        if self.rows.mail_account(address).await?.is_some() {
            return Ok(false);
        }

        self.rows.insert_mail_account(address, password).await?;

        Ok(true)
    }

    /// Removes an account when a fixture deletes it while a MetaMail object is still alive.
    pub async fn delete_mail_account(&self, address: &str) -> Result<()> {
        self.rows.remove_mail_account(address).await
    }

    pub async fn mail_login(&self, address: &str, password: &str) -> Result<Login> {
        let valid = self
            .rows
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

    pub async fn is_mail_account_disabled(&self, address: &str) -> Result<bool> {
        self.is_mail_disabled(address).await
    }

    pub async fn is_mail_disabled(&self, address: &str) -> Result<bool> {
        let Some(person) = self.rows.mail_owner(address).await? else {
            return Ok(false);
        };

        Ok(self
            .npcs
            .npc(person)
            .await?
            .is_some_and(|npc| npc.is_dead_or_fired(self.world.time_seconds())))
    }

    pub async fn restore_mail_account(
        &self,
        address: &str,
        password: &str,
        generated: bool,
        emails: &[Email],
    ) -> Result<()> {
        self.rows.insert_mail_account(address, password).await?;
        self.rows.clear_inbox(address).await?;

        for email in emails.iter().rev() {
            let id = self
                .rows
                .insert_email(address, &email.from, &email.subject, &email.body)
                .await?;

            for reply in &email.replies {
                self.rows.insert_reply(address, &id, reply).await?;
            }
        }

        self.rows.set_npc_contents(address, generated).await
    }
}
