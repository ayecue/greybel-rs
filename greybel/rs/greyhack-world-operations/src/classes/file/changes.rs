//! File copy, move, symlink, rename, and delete operations (`FileIntrinsics`).

use greyhack_world::{DeletionOrigin, Node, World};

use super::{accessible_resolve, file_change, file_failure, resolve};
use crate::Result;
use crate::connection::objects::FileRef;
use crate::protocol::{Failure, FileChange, FileFailure, Outcome};

/// Copies a file to `folder`/`name` using `PlayerUtils.MoverFichero` validation.
///
/// Copies commit synchronously. The protocol and work model have no download start/result operation or
/// HDD stability job, so transfer time, progress, deferred HDD errors, and `WaitResultDownload` from
/// `FileIntrinsics.copy` are intentionally not simulated.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn copy(
    world: &mut World,
    file: &FileRef,
    folder: &str,
    name: &str,
) -> Result<(Outcome, Option<Node>)> {
    let Some(node) = accessible_resolve(world, file, false).await? else {
        return Ok((Outcome::Null, None));
    };

    Ok(
        match world
            .copy_entry(&node, &file.access.user, folder, name)
            .await?
        {
            Ok(copy) => {
                let replacement = (world.node(file.access.computer, file.node).await?.is_none())
                    .then_some(copy.node);
                let outcome = if copy.skipped {
                    file_failure(FileFailure::PartialCopy)
                } else {
                    Outcome::True
                };

                (outcome, replacement)
            }
            Err(failure) => (Outcome::Failure(Failure::try_from(failure)?), None),
        },
    )
}

/// Moves a file to `folder`/`name` and returns the moved node on success.
///
/// # Errors
///
/// Returns [`crate::protocol::ScriptFailure::TooManyFolders`] when `folder` has more than 16 folders, or a
/// database read/write failure.
pub async fn r#move(
    world: &mut World,
    file: &FileRef,
    folder: &str,
    name: &str,
) -> Result<(Outcome, Option<Node>)> {
    let Some(node) = accessible_resolve(world, file, false).await? else {
        return Ok((Outcome::Null, None));
    };

    Ok(
        match world
            .move_entry(&node, &file.access.user, folder, name)
            .await?
        {
            Ok(node) => (Outcome::True, Some(node)),
            Err(failure) => (Outcome::Failure(Failure::try_from(failure)?), None),
        },
    )
}

/// Creates `folder`/`name` as a symlink to the file.
///
/// # Errors
///
/// Returns [`crate::protocol::ScriptFailure::TooManyFolders`] when `folder` has more than 16 folders, or a
/// database read/write failure.
pub async fn symlink(
    world: &mut World,
    file: &FileRef,
    folder: &str,
    name: &str,
) -> Result<Outcome> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(Outcome::Null);
    };

    crate::classes::creation_outcome(
        world
            .symlink_entry(&node, &file.access.user, folder, name)
            .await?,
    )
}

/// Renames a file and returns success or the game error, with the new node on success.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn rename(
    world: &mut World,
    file: &FileRef,
    name: &str,
) -> Result<(FileChange, Option<Node>)> {
    let Some(node) = resolve(world, file, false).await? else {
        return Ok(if world.computer(file.access.computer).await?.is_none() {
            (FileChange::Null, None)
        } else {
            (
                file_change(FileFailure::MissingFile(file.path.clone())),
                None,
            )
        });
    };

    Ok(
        match world.rename_entry(&node, &file.access.user, name).await? {
            Ok(node) => (FileChange::Done, Some(node)),
            Err(failure) => (FileChange::Failure(Failure::try_from(failure)?), None),
        },
    )
}

/// Deletes the referenced file.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete(world: &mut World, file: &FileRef, terminal: u64) -> Result<FileChange> {
    let computer = file.access.computer;

    if world.computer(computer).await?.is_none() {
        return Ok(FileChange::Null);
    }

    let Some(node) = resolve(world, file, false).await? else {
        return Ok(file_change(FileFailure::MissingFile(file.path.clone())));
    };
    let origin = DeletionOrigin {
        prev_ip: &file.access.prev_ip,
        path: &file.path,
        bounces: file.access.bounces,
        terminal,
    };

    Ok(
        match world
            .delete_entry(&node, &file.access.user, &origin)
            .await?
        {
            Ok(true) => file_change(FileFailure::PartialDelete),
            Ok(false) => FileChange::Done,
            Err(failure) => FileChange::Failure(Failure::try_from(failure)?),
        },
    )
}
