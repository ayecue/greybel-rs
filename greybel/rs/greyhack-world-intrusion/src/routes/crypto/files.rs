//! File encryption (`CryptoIntrinsics.encrypt`, `decrypt`, `is_encrypted`).

use greyhack_world_core::Access;

use crate::routes::Result;
use crate::routes::protocol::{IntoProtocol as _, Outcome};
use crate::{CryptoFailure, IntrusionContext};

/// Converts a file operation result to an outcome: `Null` when the computer is gone.
pub(super) fn file_outcome(result: Option<std::result::Result<(), CryptoFailure>>) -> Outcome {
    match result {
        None => Outcome::Null,
        Some(Ok(())) => Outcome::True,
        Some(Err(failure)) => Outcome::Failure(failure.into_protocol()),
    }
}

/// Encrypts `path` using `password`.
pub(super) async fn encrypt(
    world: &impl IntrusionContext,
    access: &Access,
    path: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(file_outcome(
        world
            .intrusion()
            .crypto
            .encrypt_file(access.computer, &access.user, path, password)
            .await?,
    ))
}

/// Decrypts `path` after verifying `password`.
pub(super) async fn decrypt(
    world: &impl IntrusionContext,
    access: &Access,
    path: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(file_outcome(
        world
            .intrusion()
            .crypto
            .decrypt_file(access.computer, &access.user, path, password)
            .await?,
    ))
}

/// Checks `path` for encryption, returning any read failure.
pub(super) async fn is_encrypted(
    world: &impl IntrusionContext,
    access: &Access,
    path: &str,
) -> Result<Outcome> {
    Ok(
        match world
            .intrusion()
            .crypto
            .is_file_encrypted(access.computer, &access.user, path)
            .await?
        {
            None => Outcome::Null,
            Some(Ok(is_encrypted)) => is_encrypted.into_protocol(),
            Some(Err(failure)) => Outcome::Failure(failure.into_protocol()),
        },
    )
}
