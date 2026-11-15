//! File content and directory-entry operations (`FileIntrinsics`).

use greyhack_world_router::protocol::{Failure, FileContent, Outcome};

use super::{EntryResult, accessible_resolve};
use crate::Node;
use crate::objects::FileRef;
use crate::routes::conversions::IntoProtocol as _;
use crate::routes::{OperationError, Result};

/// Lists either subdirectories or files in a directory.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn entries(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    folders: bool,
) -> Result<EntryResult> {
    let Some(node) = accessible_resolve(world, file, true)
        .await?
        .filter(Node::is_folder)
    else {
        return Ok(EntryResult::Null);
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .list_entries(&node, &file.access.user, &file.path, folders)
            .await?
        {
            Ok(nodes) => EntryResult::Nodes(nodes),
            Err(failure) => EntryResult::Failure(Failure::File(failure.into_protocol())),
        },
    )
}

/// Returns file text or the read failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn content(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
) -> Result<FileContent> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(FileContent::Null);
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .read_content(&node, &file.access.user, &file.path)
            .await?
        {
            Ok(content) => FileContent::Content(content),
            Err(failure) => FileContent::Failure(
                <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                    .map_err(OperationError::Script)?,
            ),
        },
    )
}

/// Replaces file text and updates its timestamp.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn set_content(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    content: &str,
) -> Result<Outcome> {
    let Some(node) = accessible_resolve(world, file, true).await? else {
        return Ok(Outcome::Null);
    };

    Ok(
        match world
            .module::<crate::CoreModule>()
            .files
            .scripts
            .write_content(&node, &file.access.user, &file.path, content)
            .await?
        {
            Ok(can_write) => can_write.into_protocol(),
            Err(failure) => Outcome::Failure(
                <Failure as crate::routes::conversions::TryProtocolFrom<_>>::try_from(failure)
                    .map_err(OperationError::Script)?,
            ),
        },
    )
}
