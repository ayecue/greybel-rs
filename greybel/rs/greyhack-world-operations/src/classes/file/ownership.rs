//! File permission and ownership changes (`FileIntrinsics`).

use greyhack_behaviour::FileOwnership;
use greyhack_world::World;

use super::{accessible_resolve, file_change};
use crate::Result;
use crate::connection::objects::FileRef;
use crate::protocol::{FileChange, FileFailure, ScriptFailure};

/// Applies a `chmod` expression, optionally recursively.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn chmod(
    world: &mut World,
    file: &FileRef,
    change: &str,
    recursive: bool,
) -> Result<FileChange> {
    let computer = file.access.computer;

    if world.computer(computer).await?.is_none() {
        return Ok(FileChange::Null);
    }

    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(file_change(FileFailure::NoPermission));
    };
    let account = world.user(computer, &file.access.user).await?;

    Ok(
        match world
            .change_permissions(&node, change, account.as_ref(), recursive)
            .await?
        {
            Ok(()) => FileChange::Done,
            Err(failure) => file_change(failure.into()),
        },
    )
}

/// Changes owner or group with `UpdateOwnerAbsolute` or `UpdateGroupAbsolute` behavior.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn set_owner(
    world: &mut World,
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
            return Err(ScriptFailure::InvalidArguments.into());
        }
    }

    let account = world.user(node.computer, &file.access.user).await?;

    Ok(
        match world
            .set_owner(&node, owner, group, account.as_ref(), recursive)
            .await?
        {
            Ok(()) => FileChange::Done,
            Err(failure) => file_change(failure.into()),
        },
    )
}
