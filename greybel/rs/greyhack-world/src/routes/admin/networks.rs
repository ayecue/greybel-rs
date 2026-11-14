//! Program creation through the global `admin.so` program.

use greyhack_behaviour::{FileData, PermissionSet, path};

use crate::routes::Result;
use crate::{ComputerId, NodeDraft, World};

/// Creates script `source` at `path` on `computer` with owner `user`.
///
/// # Errors
///
/// Returns an error when the world database refuses a read.
pub(super) async fn create_program(
    world: &World,
    computer: ComputerId,
    user: &str,
    path: &str,
    source: &str,
) -> Result<bool> {
    let Some(folder) = world
        .modules
        .core
        .files
        .lookup
        .folder(computer, path::parent(path), true)
        .await?
    else {
        return Ok(false);
    };
    let node = NodeDraft::file(
        path::file_name(path),
        user,
        PermissionSet::default_for(false),
        FileData::script(source),
    );

    world
        .modules
        .libraries
        .generator
        .add_full_node(&folder, &node)
        .await?;

    Ok(true)
}
