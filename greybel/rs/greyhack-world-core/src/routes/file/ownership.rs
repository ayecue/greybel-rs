//! File permission and ownership changes (`FileIntrinsics`).

use greyhack_behaviour::FileOwnership;
use greyhack_world_programs::protocol::{FileChange, FileFailure, ScriptFailure};

use super::{accessible_resolve, file_change};
use crate::objects::FileRef;
use crate::routes::conversions::IntoProtocol as _;
use crate::routes::{OperationError, Result};

/// Applies a `chmod` expression, optionally recursively.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn chmod(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    change: &str,
    recursive: bool,
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

    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(file_change(FileFailure::NoPermission));
    };
    let account = world
        .module::<crate::CoreModule>()
        .computers
        .users
        .user(computer, &file.access.user)
        .await?;

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .permissions
            .change_permissions(&node, change, account.as_ref(), recursive)
            .await?
        {
            Ok(()) => FileChange::Done,
            Err(failure) => file_change(failure.into_protocol()),
        },
    )
}

/// Changes owner or group with `UpdateOwnerAbsolute` or `UpdateGroupAbsolute` behavior.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn set_owner(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    owner: Option<&str>,
    group: Option<&str>,
    recursive: bool,
) -> Result<FileChange> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(FileChange::Null);
    };

    for (holder, name) in [(FileOwnership::Owner, owner), (FileOwnership::Group, group)] {
        if name.is_some_and(|name| holder.check(name).is_err()) {
            return Err(OperationError::Script(ScriptFailure::InvalidArguments));
        }
    }

    let account = world
        .module::<crate::CoreModule>()
        .computers
        .users
        .user(node.computer, &file.access.user)
        .await?;

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .permissions
            .set_owner(&node, owner, group, account.as_ref(), recursive)
            .await?
        {
            Ok(()) => FileChange::Done,
            Err(failure) => file_change(failure.into_protocol()),
        },
    )
}
