//! Remote shells a script opens with `connect_service` (`ShellIntrinsics.connect_service`,
//! `PlayerUtils.ConnectService`).

use greyhack_behaviour::{FileData, FileKind, LogAction, SSHD_CONFIG_PATH};
use greyhack_world_services::SshdConfig;

use crate::{ComputerId, LoginFailure, Result};

/// Where and as whom a script logs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceRequest<'a> {
    /// Address of the router.
    pub address: &'a str,
    /// Port of the service.
    pub port: u16,
    /// Service name.
    pub service: &'a str,
    /// User to log in as.
    pub user: &'a str,
    /// Password of the user.
    pub password: &'a str,
}

/// An encode or decode script of an encrypted login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptionScript {
    /// Path of the script.
    pub path: String,
    /// Source of the script.
    pub source: String,
}

/// Encryption of the login to a computer (`PlayerUtils.GetEncryptionFiles`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encryption {
    /// Whether the connection shows the encryption message.
    pub message: bool,
    /// Script that encodes the password.
    pub encode: EncryptionScript,
    /// Script that decodes the password.
    pub decode: EncryptionScript,
}

/// What opening a service connection did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceOpen {
    /// The script's computer no longer exists.
    Missing,
    /// The script's computer has no network access.
    Offline,
    /// The connection failed before reaching a computer.
    Failure(LoginFailure),
    /// A computer was reached.
    Connection {
        /// The computer reached.
        target: ComputerId,
        /// Public address of the script's computer.
        origin: String,
        /// Why the login failed, if it did.
        failure: Option<LoginFailure>,
        /// The login needs the client's encode and decode scripts.
        encryption: Option<Encryption>,
    },
}

/// Remote service logins and encrypted handshakes.
impl ConnectionService {
    pub async fn open_service(
        &self,
        from: ComputerId,
        prev_ip: &str,
        terminal_computers: &[ComputerId],
        request: &ServiceRequest<'_>,
    ) -> Result<ServiceOpen> {
        if self.networks.topology.computer(from).await?.is_none() {
            return Ok(ServiceOpen::Missing);
        }

        if !self
            .networks
            .reach
            .is_network_active(from, terminal_computers)
            .await?
        {
            return Ok(ServiceOpen::Offline);
        }

        let ServiceRequest {
            address,
            port,
            service,
            user,
            password,
        } = *request;
        let handshake = match self
            .networks
            .connect
            .connect_service_handshake(
                from,
                address,
                port,
                service,
                user,
                password,
                terminal_computers,
            )
            .await?
        {
            Ok(handshake) => handshake,
            Err(failure) => return Ok(ServiceOpen::Failure(failure)),
        };
        let origin = self.networks.topology.public_ip(from).await?;

        // `PlayerUtils.ConnectToService` logs the login on the reached computer once the password is
        // right.
        if handshake.result.is_ok() {
            self.computers
                .logs
                .write_open_log(handshake.target, &origin, port)
                .await?;
        }

        let failure = handshake.result.err();
        let encryption = self.encryption_of(handshake.target).await?;

        if encryption.is_some() {
            return Ok(ServiceOpen::Connection {
                target: handshake.target,
                origin,
                failure,
                encryption,
            });
        }

        // `PlayerUtils.ConnectService` records plaintext passwords for all reached computers without
        // encryption, including failed logins.
        self.sniffers
            .record_service_packet(from, handshake.target, address, port, user, password)
            .await?;

        // `PlayerUtils.ConnectService` logs the new shell once the login succeeded.
        if failure.is_none() {
            self.computers
                .logs
                .write_bounce_logs(prev_ip, port, from, handshake.target)
                .await?;
            self.traces.trace_connection(handshake.target).await?;
        }

        Ok(ServiceOpen::Connection {
            target: handshake.target,
            origin,
            failure,
            encryption: None,
        })
    }

    pub async fn retry_service_login(
        &self,
        from: ComputerId,
        terminal_computers: &[ComputerId],
        request: &ServiceRequest<'_>,
    ) -> Result<Option<LoginFailure>> {
        let origin = self.networks.topology.public_ip(from).await?;
        let ServiceRequest {
            address,
            port,
            service,
            user,
            password,
        } = *request;

        Ok(
            match self
                .networks
                .connect
                .connect_service_handshake(
                    from,
                    address,
                    port,
                    service,
                    user,
                    password,
                    terminal_computers,
                )
                .await?
            {
                Ok(handshake) => {
                    if handshake.result.is_ok() {
                        self.computers
                            .logs
                            .write_open_log(handshake.target, &origin, port)
                            .await?;
                    }

                    handshake.result.err()
                }
                Err(failure) => Some(failure),
            },
        )
    }

    pub async fn finish_encrypted_login(
        &self,
        (from, prev_ip): (ComputerId, &str),
        target: ComputerId,
        request: &ServiceRequest<'_>,
        encoded_password: &str,
        failed: bool,
    ) -> Result<()> {
        if !encoded_password.is_empty() {
            self.sniffers
                .record_encrypted_service_packet(
                    from,
                    target,
                    request.address,
                    request.port,
                    request.user,
                    encoded_password,
                )
                .await?;
        }

        if !failed {
            self.computers
                .logs
                .write_bounce_logs(prev_ip, request.port, from, target)
                .await?;
            self.traces.trace_connection(target).await?;
        }

        Ok(())
    }

    pub async fn trace_shell(
        &self,
        computer: ComputerId,
        bounces: u32,
        terminal: u64,
    ) -> Result<bool> {
        self.traces
            .trace_action(computer, bounces, LogAction::Shell, terminal)
            .await
    }
}

impl ConnectionService {
    pub async fn encryption_of(&self, target: ComputerId) -> Result<Option<Encryption>> {
        let Some(config_file) = self
            .files
            .lookup
            .file(target, SSHD_CONFIG_PATH, true)
            .await?
        else {
            return Ok(None);
        };
        let Some(config_data) = config_file.data().filter(|data| !data.is_binary()) else {
            return Ok(None);
        };
        let Some(config) = SshdConfig::parse(&config_data.content)
            .filter(|config| !config.path_enc.is_empty() && !config.path_dec.is_empty())
        else {
            return Ok(None);
        };
        let (Some(encode_file), Some(decode_file)) = (
            self.files
                .lookup
                .file(target, &config.path_enc, true)
                .await?,
            self.files
                .lookup
                .file(target, &config.path_dec, true)
                .await?,
        ) else {
            return Ok(None);
        };
        let Some(encode) = encode_file.data().filter(|data| !FileData::is_binary(data)) else {
            return Ok(None);
        };
        let Some(decode) = decode_file
            .data()
            .filter(|data| data.kind == FileKind::Program)
        else {
            return Ok(None);
        };

        Ok(Some(Encryption {
            message: config.message_encrypted_conn,
            encode: EncryptionScript {
                path: config.path_enc,
                source: encode.content.clone(),
            },
            decode: EncryptionScript {
                path: config.path_dec,
                source: decode.content.clone(),
            },
        }))
    }
}

/// A world service with only its declared dependencies.
pub struct ConnectionService {
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) traces: greyhack_world_core::Dep<greyhack_world_traces::TraceService>,
    pub(crate) sniffers: greyhack_world_core::Dep<greyhack_world_intrusion::SnifferService>,
}

impl ConnectionService {
    pub(crate) fn new(
        computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
        traces: greyhack_world_core::Dep<greyhack_world_traces::TraceService>,
        sniffers: greyhack_world_core::Dep<greyhack_world_intrusion::SnifferService>,
    ) -> Self {
        Self {
            computers,
            files,
            networks,
            traces,
            sniffers,
        }
    }
}
