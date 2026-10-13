//! Packets sniffers record from logins and service connections.

use greyhack_behaviour::SSHD_CONFIG_PATH;

use crate::model::{ComputerId, SnifferPacket};
use crate::{DEFAULT_ENCODE_PATH, Result, SshdConfig, World};

struct LoginPacket<'a> {
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

impl World {
    pub(crate) async fn record_packet(
        &mut self,
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

    /// Records a completed unencrypted service login. `encPass` is captured only after encrypted
    /// encode/decode succeeds (`PlayerUtils.ConnectService`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn record_service_packet(
        &mut self,
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

    /// Records an encrypted login using client-supplied encoder output after both SSH scripts
    /// succeed.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn record_encrypted_service_packet(
        &mut self,
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

    async fn record_packet_impl(
        &mut self,
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
                    .content(destination, SSHD_CONFIG_PATH, true)
                    .await?
                    .and_then(|content| SshdConfig::parse(&content))
                    .map_or_else(|| DEFAULT_ENCODE_PATH.to_owned(), |config| config.path_enc);

                self.content(destination, &path, true).await?
            }
            Some(false) | None => self.content(destination, DEFAULT_ENCODE_PATH, true).await?,
        };
        // Login source includes the computer public and LAN addresses
        // (`PlayerUtils.ConnectService`).
        let source = match self.db.computer(source).await? {
            Some(computer) => format!(
                "{}/{}",
                self.public_ip(computer.id).await?,
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
