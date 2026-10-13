//! File content and directory-entry operations (`FileIntrinsics`).

use greyhack_world::{Node, World};

use super::{EntryResult, accessible_resolve};
use crate::Result;
use crate::connection::objects::FileRef;
use crate::protocol::{Failure, FileContent, Outcome};

/// Lists either subdirectories or files in a directory.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn entries(world: &World, file: &FileRef, folders: bool) -> Result<EntryResult> {
    let Some(node) = accessible_resolve(world, file, true)
        .await?
        .filter(Node::is_folder)
    else {
        return Ok(EntryResult::Null);
    };

    Ok(
        match world
            .list_entries(&node, &file.access.user, &file.path, folders)
            .await?
        {
            Ok(nodes) => EntryResult::Nodes(nodes),
            Err(failure) => EntryResult::Failure(Failure::File(failure.into())),
        },
    )
}

/// Returns file text or the read failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn content(world: &World, file: &FileRef) -> Result<FileContent> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(FileContent::Null);
    };

    Ok(
        match world
            .read_content(&node, &file.access.user, &file.path)
            .await?
        {
            Ok(content) => FileContent::Content(content),
            Err(failure) => FileContent::Failure(Failure::try_from(failure)?),
        },
    )
}

/// Replaces file text and updates its timestamp.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn set_content(world: &mut World, file: &FileRef, content: &str) -> Result<Outcome> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(Outcome::Null);
    };

    Ok(
        match world
            .write_content(&node, &file.access.user, &file.path, content)
            .await?
        {
            Ok(can_write) => can_write.into(),
            Err(failure) => Outcome::Failure(Failure::try_from(failure)?),
        },
    )
}
