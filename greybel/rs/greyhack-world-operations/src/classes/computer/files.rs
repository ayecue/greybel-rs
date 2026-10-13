//! Computer file lookup and creation (`ComputerIntrinsics`).

use greyhack_world::{Node, ScriptFileFailure, World};

use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{Failure, Outcome};

/// Looks up `path` while without resolving its final symlink unresolved.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn file(world: &World, access: &Access, path: &str) -> Result<Option<Node>> {
    if world.user(access.computer, &access.user).await?.is_none() {
        return Ok(None);
    }

    Ok(world.entry(access.computer, path, false).await?)
}

/// Converts the result of a file creation to an operation outcome.
pub(crate) fn creation_outcome(
    result: std::result::Result<(), ScriptFileFailure>,
) -> Result<Outcome> {
    Ok(match result {
        Ok(()) => Outcome::True,
        Err(failure) => Outcome::Failure(Failure::try_from(failure)?),
    })
}

/// Creates an empty text file named `name` under `folder`.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn touch(
    world: &mut World,
    access: &Access,
    folder: &str,
    name: &str,
) -> Result<Outcome> {
    match world
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
    world: &mut World,
    access: &Access,
    folder: &str,
    name: &str,
) -> Result<Outcome> {
    match world
        .create_folder(access.computer, &access.user, folder, name)
        .await?
    {
        None => Ok(Outcome::Null),
        Some(result) => creation_outcome(result),
    }
}
