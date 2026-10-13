//! File metadata operations (`FileIntrinsics`): paths, names, types, sizes, dates, permissions, and owners.

use greyhack_behaviour::{FileData, Permission};
use greyhack_world::{Node, World};

use super::{accessible_resolve, resolve};
use crate::Result;
use crate::classes::node;
use crate::connection::objects::FileRef;

/// Returns the current path, or the symlink target when `original` is true.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn path(world: &World, file: &FileRef, original: bool) -> Result<Option<String>> {
    if world.computer(file.access.computer).await?.is_none() {
        return Ok(None);
    }

    if !original {
        return Ok(Some(file.path.clone()));
    }

    // Return symlink targets and null for removed files (`FileIntrinsics.path`).
    Ok(node(world, file).await?.map(|node| {
        if node.is_symlink() {
            node.symlink
        } else {
            file.path.clone()
        }
    }))
}

/// Looks up the parent directory node.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn parent(world: &World, file: &FileRef) -> Result<Option<Node>> {
    let Some(node) = resolve(world, file, true).await? else {
        return Ok(None);
    };
    let Some(parent) = node.parent else {
        return Ok(None);
    };

    Ok(world.node(node.computer, parent).await?)
}

/// Looks up the filename without resolving its final symlink.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn name(world: &World, file: &FileRef) -> Result<Option<String>> {
    Ok(resolve(world, file, false)
        .await?
        .map(|node| node.name().to_owned()))
}

/// Checks the resolved node for directory type.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_folder(world: &World, file: &FileRef) -> Result<Option<bool>> {
    Ok(resolve(world, file, true)
        .await?
        .map(|node| node.is_folder()))
}

/// Checks the unresolved node for symlink type.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_symlink(world: &World, file: &FileRef) -> Result<Option<bool>> {
    Ok(resolve(world, file, false)
        .await?
        .map(|node| node.is_symlink()))
}

/// Formats the file size in bytes as text.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn size(world: &World, file: &FileRef) -> Result<Option<String>> {
    let Some(node) = resolve(world, file, false).await? else {
        return Ok(None);
    };

    Ok(Some(world.size(&node, false).await?.to_string()))
}

/// Formats the unresolved node timestamp for `file.date` (`FileIntrinsics.date`).
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn date(world: &World, file: &FileRef) -> Result<Option<String>> {
    Ok(resolve(world, file, false)
        .await?
        .map(|node| greyhack_behaviour::format_file_date(node.timestamp)))
}

/// Checks whether the resolved file contains binary data.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn is_binary(world: &World, file: &FileRef) -> Result<Option<bool>> {
    Ok(resolve(world, file, true)
        .await?
        .map(|node| node.data().is_none_or(FileData::is_binary)))
}

/// Checks whether the resolved file permits import.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn allow_import(world: &World, file: &FileRef) -> Result<bool> {
    Ok(resolve(world, file, true)
        .await?
        .and_then(|node| node.data().cloned())
        .is_some_and(|data| data.allow_import))
}

/// Formats resolved file permissions as an `ls` mode string.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn permissions(world: &World, file: &FileRef) -> Result<Option<String>> {
    Ok(resolve(world, file, true)
        .await?
        .map(|node| node.permissions.to_string()))
}

/// Looks up the resolved file owner.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn owner(world: &World, file: &FileRef) -> Result<Option<String>> {
    Ok(resolve(world, file, true).await?.map(|node| node.owner))
}

/// Looks up the resolved file group.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn group(world: &World, file: &FileRef) -> Result<Option<String>> {
    Ok(resolve(world, file, true).await?.map(|node| node.group))
}

/// Checks `r`, `w`, or `x` permission for the file user; other mode letters return `None`.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn has_permission(
    world: &World,
    file: &FileRef,
    permission: &str,
) -> Result<Option<bool>> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(None);
    };
    let Some(permission) = Permission::from_mode(permission) else {
        return Ok(None);
    };

    Ok(Some(
        world
            .user_has_permission(node.computer, &file.access.user, &node, permission)
            .await?,
    ))
}
