//! Remote shell creation through `connect_service`.

use greyhack_behaviour::{forbidden_param_token, previous_address};
use greyhack_world::{
    ComputerId, Encryption, EncryptionScript, ServiceOpen, ServiceRequest, World,
};

use super::require_connection;
use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{
    Failure, LoginFailure, ScriptFailure, ShellCompleteEncryptedLogin, ShellFailure, SourceFile,
};

/// Result of a `connect_service` login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAccess {
    /// Login or connection failure.
    Failure(Failure),
    Shell(Access),
    /// Encrypted login requiring guest-side encode/decode scripts, retaining the initial login result
    /// The initial result follows `ShellIntrinsics.connect_service`.
    Encryption {
        access: Access,
        message: bool,
        failure: Option<LoginFailure>,
        encode: SourceFile,
        decode: SourceFile,
    },
}

/// Unvalidated script arguments for `connect_service`.
pub struct ServiceLogin<'a> {
    pub address: Option<&'a str>,
    pub port: Option<i32>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    pub service: Option<&'a str>,
}

/// Applies `connect_service` validation in game order and opens a remote shell.
///
/// # Errors
///
/// Returns connection, argument, network, or database search failures.
pub async fn connect_service(
    world: &mut World,
    shell: &Access,
    terminal_computers: &[ComputerId],
    login: &ServiceLogin<'_>,
) -> Result<ServiceAccess> {
    require_connection(world, terminal_computers).await?;

    let (Some(address), Some(port), Some(user), Some(password), Some(service)) = (
        login.address,
        login.port,
        login.user,
        login.password,
        login.service,
    ) else {
        return Err(ScriptFailure::InvalidArguments.into());
    };

    if address.is_empty() {
        return Err(ScriptFailure::InvalidArguments.into());
    }

    if let Some(forbidden) = forbidden_param_token(password) {
        return Ok(ServiceAccess::Failure(Failure::Shell(
            ShellFailure::InvalidPasswordCharacter(forbidden.to_owned()),
        )));
    }

    let request = ServiceRequest {
        address,
        port: u16::try_from(port).unwrap_or(0),
        service,
        user,
        password,
    };
    let (target, origin, failure, encryption) = match world
        .open_service(shell.computer, &shell.prev_ip, terminal_computers, &request)
        .await?
    {
        ServiceOpen::Gone => {
            return Ok(ServiceAccess::Failure(Failure::Shell(
                ShellFailure::ComputerUnavailable,
            )));
        }
        ServiceOpen::Offline => return Err(ScriptFailure::NoAccess.into()),
        ServiceOpen::Failure(failure) => {
            return Ok(ServiceAccess::Failure(Failure::Login(failure.into())));
        }
        ServiceOpen::Connection {
            target,
            origin,
            failure,
            encryption,
        } => (target, origin, failure.map(LoginFailure::from), encryption),
    };
    let access = Access {
        computer: target,
        user: user.to_owned(),
        ftp: service.eq_ignore_ascii_case("ftp"),
        prev_ip: previous_address(&origin, address, &shell.prev_ip),
        bounces: shell.bounces + 1,
    };

    if let Some(Encryption {
        message,
        encode,
        decode,
    }) = encryption
    {
        return Ok(ServiceAccess::Encryption {
            access,
            message,
            failure,
            encode: source_file(encode),
            decode: source_file(decode),
        });
    }

    Ok(match failure {
        Some(failure) => ServiceAccess::Failure(Failure::Login(failure)),
        None => ServiceAccess::Shell(access),
    })
}

/// Converts an encryption script to protocol form.
fn source_file(script: EncryptionScript) -> SourceFile {
    SourceFile {
        path: script.path,
        source: script.source,
    }
}

/// Completes an encrypted login after the client runs encode and decode scripts (`PlayerUtils.ConnectService`).
///
/// # Errors
///
/// Returns database search or packet-recording failures.
pub async fn complete_encrypted_login(
    world: &mut World,
    source: &Access,
    target: &Access,
    terminal_computers: &[ComputerId],
    login: &ShellCompleteEncryptedLogin<'_>,
) -> Result<std::result::Result<Access, LoginFailure>> {
    let origin = world.public_ip(source.computer).await?;
    let request = ServiceRequest {
        address: &login.address,
        port: login.port,
        service: &login.service,
        user: &login.user,
        password: &login.password,
    };
    let failure = if login.decoded_password != login.password {
        // `ShellIntrinsics.connect_service` retries authentication when
        // `HelperScript.HelperSshSec.Completed` decodes a different password, replacing the initial result.
        world
            .retry_service_login(
                source.computer,
                terminal_computers,
                &ServiceRequest {
                    password: &login.decoded_password,
                    ..request
                },
            )
            .await?
            .map(LoginFailure::from)
    } else {
        login.failure.clone()
    };

    world
        .finish_encrypted_login(
            (source.computer, &source.prev_ip),
            target.computer,
            &request,
            &login.encoded_password,
            failure.is_some(),
        )
        .await?;

    Ok(match failure {
        Some(failure) => Err(failure),
        None => Ok(Access {
            computer: target.computer,
            user: login.user.to_string(),
            ftp: login.service.eq_ignore_ascii_case("ftp"),
            prev_ip: previous_address(&origin, &login.address, &source.prev_ip),
            bounces: source.bounces + 1,
        }),
    })
}
