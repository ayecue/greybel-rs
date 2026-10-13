//! Logins NPCs make over the network that path sniffers capture.

use greyhack_behaviour::ServiceId;

use crate::model::{ComputerId, SnifferPacket};
use crate::{Result, World};

impl World {
    /// Runs the social-engineering login flow: an NPC logs into a service, exposes credentials to
    /// path sniffers, and remains logged in for one hour unless already online
    /// (`Persona.ProcesaIngenieraSocial`, `LoginService` mail). Dead NPCs do not respond; scheduled
    /// logins emit no credentials.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn npc_social_login(&mut self, computer: ComputerId, user: &str) -> Result<()> {
        let npc = self.npc_on_computer(computer, user).await?;

        if npc
            .as_ref()
            .is_some_and(|npc| npc.is_dead(self.time_seconds))
        {
            return Ok(());
        }

        self.record_npc_login(computer, user).await?;

        match npc {
            Some(npc) if !npc.is_logged(self.time_seconds) => self.force_npc_login(&npc).await,
            _ => Ok(()),
        }
    }

    /// Captures NPC `user` login on `computer` over SSH, otherwise FTP, for path sniffers.
    pub(crate) async fn record_npc_login(
        &mut self,
        computer: ComputerId,
        user: &str,
    ) -> Result<()> {
        let Some(row) = self.db.computer(computer).await? else {
            return Ok(());
        };
        let Some(router) = row.router else {
            return Ok(());
        };

        let protocol = if self
            .db
            .service_port(computer, ServiceId::Ssh)
            .await?
            .is_some()
        {
            "SSH"
        } else if self
            .db
            .service_port(computer, ServiceId::Ftp)
            .await?
            .is_some()
        {
            "FTP"
        } else {
            return Ok(());
        };
        let Some(password) = self
            .db
            .user(computer, user)
            .await?
            .map(|user| user.password)
        else {
            return Ok(());
        };
        let path = self
            .login_path(router.computer(), computer, &row.local_ip)
            .await?;
        let sniffers = self.sniffers_on_path(&path).await?;

        if sniffers.is_empty() {
            return Ok(());
        }

        let packet = SnifferPacket {
            source: self.public_ip(computer).await?,
            destination: row.local_ip.clone(),
            protocol: protocol.to_owned(),
            user: user.to_owned(),
            password,
            encrypted: false,
            encoder: None,
        };

        self.capture(&sniffers, &packet).await
    }
}
