//! Packets sniffers record from logins and service connections.

use greyhack_behaviour::SSHD_CONFIG_PATH;
use greyhack_world_core::{ComputerId, Result};
use greyhack_world_services::{DEFAULT_ENCODE_PATH, SshdConfig};

use crate::{SnifferPacket, Sniffers};

pub(crate) struct LoginPacket<'a> {
    source: ComputerId,
    destination: ComputerId,
    address: &'a str,
    port: u16,
    user: &'a str,
    password: &'a str,
}

impl<'a> LoginPacket<'a> {
    const fn new(
        source: ComputerId,
        destination: ComputerId,
        address: &'a str,
        port: u16,
        user: &'a str,
        password: &'a str,
    ) -> Self {
        Self {
            source,
            destination,
            address,
            port,
            user,
            password,
        }
    }
}

/// Methods of [`World`] for intrusion sniffer packets.
impl Sniffers {
    pub async fn record_packet(
        &self,
        source: ComputerId,
        destination: ComputerId,
        address: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.record_packet_impl(
            LoginPacket::new(source, destination, address, port, user, password),
            None,
        )
        .await
    }

    pub async fn record_service_packet(
        &self,
        source: ComputerId,
        destination: ComputerId,
        address: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.record_packet_impl(
            LoginPacket::new(source, destination, address, port, user, password),
            Some(false),
        )
        .await
    }

    pub async fn record_encrypted_service_packet(
        &self,
        source: ComputerId,
        destination: ComputerId,
        address: &str,
        port: u16,
        user: &str,
        password: &str,
    ) -> Result<()> {
        self.record_packet_impl(
            LoginPacket::new(source, destination, address, port, user, password),
            Some(true),
        )
        .await
    }
}

impl Sniffers {
    pub(crate) async fn record_packet_impl(
        &self,
        packet: LoginPacket<'_>,
        encryption: Option<bool>,
    ) -> Result<()> {
        let LoginPacket {
            source,
            destination,
            address,
            port,
            user,
            password,
        } = packet;
        let path = self.login_path(source, destination, address).await?;
        let sniffers = self.sniffers_on_path(&path).await?;

        if sniffers.is_empty() {
            return Ok(());
        }

        let encoder = match encryption {
            Some(true) => {
                let path = self
                    .files
                    .lookup
                    .content(destination, SSHD_CONFIG_PATH, true)
                    .await?
                    .and_then(|content| SshdConfig::parse(&content))
                    .map_or_else(|| DEFAULT_ENCODE_PATH.to_owned(), |config| config.path_enc);

                self.files.lookup.content(destination, &path, true).await?
            }
            Some(false) | None => {
                self.files
                    .lookup
                    .content(destination, DEFAULT_ENCODE_PATH, true)
                    .await?
            }
        };
        // Login source includes the computer public and LAN addresses
        // (`PlayerUtils.ConnectService`).
        let source = match self.computers.computer(source).await? {
            Some(computer) => format!(
                "{}/{}",
                self.networks.topology.public_ip(computer.id).await?,
                computer.local_ip
            ),
            None => String::new(),
        };
        let packet = SnifferPacket {
            source,
            destination: address.to_owned(),
            protocol: port.to_string(),
            user: user.to_owned(),
            password: password.to_owned(),
            encrypted: encryption.unwrap_or_else(|| encoder.is_some()),
            encoder,
        };

        self.capture(&sniffers, &packet).await
    }
}
