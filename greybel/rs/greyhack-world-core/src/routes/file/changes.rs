//! File copy, move, symlink, rename, and delete operations (`FileIntrinsics`).

use greyhack_world_programs::protocol::{Failure, FileChange, FileFailure, Outcome};

use super::{accessible_resolve, file_change, file_failure, resolve};
use crate::objects::FileRef;
use crate::routes::{OperationError, Result};
use crate::{ComputerId, DeletionOrigin, Node};

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
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    folder: &str,
    name: &str,
) -> Result<(Outcome, Option<Node>)> {
    let Some(node) = accessible_resolve(world, file, false).await? else {
        return Ok((Outcome::Null, None));
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .copy_entry(&node, &file.access.user, folder, name)
            .await?
        {
            Ok(copy) => {
                let replacement = (world
                    .module::<crate::CoreModule>()
                    .files
                    .lookup
                    .node(file.access.computer, file.node)
                    .await?
                    .is_none())
                .then_some(copy.node);
                let outcome = if copy.skipped {
                    file_failure(FileFailure::PartialCopy)
                } else {
                    Outcome::True
                };

                (outcome, replacement)
            }
            Err(failure) => (
                Outcome::Failure(
                    <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                        .map_err(OperationError::Script)?,
                ),
                None,
            ),
        },
    )
}

/// Moves a file to `folder`/`name` and returns the moved node on success.
///
/// # Errors
///
/// Returns [`greyhack_world_programs::protocol::ScriptFailure::TooManyFolders`] when `folder` has more than 16 folders, or a
/// database read/write failure.
pub async fn r#move(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    folder: &str,
    name: &str,
) -> Result<(Outcome, Option<Node>)> {
    let Some(node) = accessible_resolve(world, file, false).await? else {
        return Ok((Outcome::Null, None));
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .move_entry(&node, &file.access.user, folder, name)
            .await?
        {
            Ok(node) => (Outcome::True, Some(node)),
            Err(failure) => (
                Outcome::Failure(
                    <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                        .map_err(OperationError::Script)?,
                ),
                None,
            ),
        },
    )
}

/// Creates `folder`/`name` as a symlink to the file.
///
/// # Errors
///
/// Returns [`greyhack_world_programs::protocol::ScriptFailure::TooManyFolders`] when `folder` has more than 16 folders, or a
/// database read/write failure.
pub async fn symlink(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    folder: &str,
    name: &str,
) -> Result<Outcome> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(Outcome::Null);
    };

    crate::routes::computer::creation_outcome(
        world
            .module::<crate::CoreModule>()
            .files
            .scripts
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
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    name: &str,
) -> Result<(FileChange, Option<Node>)> {
    let Some(node) = resolve(world, file, false).await? else {
        return Ok(
            if world
                .module::<crate::CoreModule>()
                .networks
                .topology
                .computer(file.access.computer)
                .await?
                .is_none()
            {
                (FileChange::Null, None)
            } else {
                (
                    file_change(FileFailure::MissingFile(file.path.clone())),
                    None,
                )
            },
        );
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .rename_entry(&node, &file.access.user, name)
            .await?
        {
            Ok(node) => (FileChange::Done, Some(node)),
            Err(failure) => (
                FileChange::Failure(
                    <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                        .map_err(OperationError::Script)?,
                ),
                None,
            ),
        },
    )
}

/// Deletes the referenced file.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    terminal: u64,
    last_shell: ComputerId,
) -> Result<FileChange> {
    let computer = file.access.computer;

    if world
        .module::<crate::CoreModule>()
        .networks
        .topology
        .computer(computer)
        .await?
        .is_none()
    {
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
        last_shell,
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .delete_entry(&node, &file.access.user, &origin)
            .await?
        {
            Ok(true) => file_change(FileFailure::PartialDelete),
            Ok(false) => FileChange::Done,
            Err(failure) => FileChange::Failure(
                <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                    .map_err(OperationError::Script)?,
            ),
        },
    )
}
