//! Computer file lookup and creation (`ComputerIntrinsics`).

use greyhack_world_router::protocol::{Failure, Outcome};

use crate::objects::Access;
use crate::routes::{OperationError, Result};
use crate::{Node, ScriptFileFailure};

/// Looks up `path` while without resolving its final symlink unresolved.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn file(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    path: &str,
) -> Result<Option<Node>> {
    if world
        .module::<crate::CoreModule>()
        .computers
        .users
        .user(access.computer, &access.user)
        .await?
        .is_none()
    {
        return Ok(None);
    }

    Ok(world
        .module::<crate::CoreModule>()
        .files
        .lookup
        .entry(access.computer, path, false)
        .await?)
}

/// Converts the result of a file creation to an operation outcome.
pub fn creation_outcome(result: std::result::Result<(), ScriptFileFailure>) -> Result<Outcome> {
    Ok(match result {
        Ok(()) => Outcome::True,
        Err(failure) => Outcome::Failure(
            <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                .map_err(OperationError::Script)?,
        ),
    })
}

/// Creates an empty text file named `name` under `folder`.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn touch(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    folder: &str,
    name: &str,
) -> Result<Outcome> {
    match world
        .module::<crate::CoreModule>()
        .files
        .scripts
        .touch_file(access.computer, &access.user, folder, name)
        .await?
    {
        None => Ok(Outcome::Null),
        Some(result) => creation_outcome(result),
    }
}

/// Creates a directory named `name` under `folder`.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn create_folder(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    access: &Access,
    folder: &str,
    name: &str,
) -> Result<Outcome> {
    match world
        .module::<crate::CoreModule>()
        .files
        .scripts
        .create_folder(access.computer, &access.user, folder, name)
        .await?
    {
        None => Ok(Outcome::Null),
        Some(result) => creation_outcome(result),
    }
}
