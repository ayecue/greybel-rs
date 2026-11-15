//! Logins NPCs make over the network that path sniffers capture.

use greyhack_behaviour::ServiceId;
use greyhack_world_core::{ComputerId, Result};

use crate::{SnifferPacket, SnifferService};

/// Methods of [`World`] for intrusion sniffer npc logins.
impl SnifferService {
    pub async fn record_npc_login(&self, computer: ComputerId, user: &str) -> Result<()> {
        let Some(row) = self.computers.computer(computer).await? else {
            return Ok(());
        };
        let Some(router) = row.router else {
            return Ok(());
        };

        let protocol = if self
            .computers
            .service_port(computer, ServiceId::Ssh)
            .await?
            .is_some()
        {
            "SSH"
        } else if self
            .computers
            .service_port(computer, ServiceId::Ftp)
            .await?
            .is_some()
        {
            "FTP"
        } else {
            return Ok(());
        };
        let Some(password) = self
            .computers
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
            source: self.networks.topology.public_ip(computer).await?,
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

impl greyhack_world_npcs::NpcLoginCapture for SnifferService {
    fn record_npc_login<'a>(
        &'a self,
        computer: ComputerId,
        user: &'a str,
    ) -> greyhack_world_core::CallFuture<'a, ()> {
        Box::pin(async move { SnifferService::record_npc_login(self, computer, user).await })
    }
}
