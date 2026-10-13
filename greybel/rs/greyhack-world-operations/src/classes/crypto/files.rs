//! File encryption (`CryptoIntrinsics.encrypt`, `decrypt`, `is_encrypted`).

use greyhack_world::{CryptoFailure, World};

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::Outcome;

/// Converts a file operation result to an outcome: `Null` when the computer is gone.
pub(super) fn file_outcome(result: Option<std::result::Result<(), CryptoFailure>>) -> Outcome {
    match result {
        None => Outcome::Null,
        Some(Ok(())) => Outcome::True,
        Some(Err(failure)) => Outcome::Failure(failure.into()),
    }
}

/// Encrypts `path` using `password`.
pub(super) async fn encrypt(
    world: &mut World,
    access: &Access,
    path: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(file_outcome(
        world
            .encrypt_file(access.computer, &access.user, path, password)
            .await?,
    ))
}

/// Decrypts `path` after verifying `password`.
pub(super) async fn decrypt(
    world: &mut World,
    access: &Access,
    path: &str,
    password: &str,
) -> Result<Outcome> {
    Ok(file_outcome(
        world
            .decrypt_file(access.computer, &access.user, path, password)
            .await?,
    ))
}

/// Checks `path` for encryption, returning any read failure.
pub(super) async fn is_encrypted(world: &World, access: &Access, path: &str) -> Result<Outcome> {
    Ok(
        match world
            .is_file_encrypted(access.computer, &access.user, path)
            .await?
        {
            None => Outcome::Null,
            Some(Ok(is_encrypted)) => is_encrypted.into(),
            Some(Err(failure)) => Outcome::Failure(failure.into()),
        },
    )
}
