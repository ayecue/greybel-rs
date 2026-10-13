//! File-object operations (`FileIntrinsics`) with current-path resolution per call.

mod changes;
mod content;
mod ownership;
mod properties;

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{Node, World};

use self::changes::{copy, delete, r#move, rename, symlink};
use self::content::{content, entries, set_content};
use self::ownership::{chmod, set_owner};
use self::properties::{
    allow_import, date, group, has_permission, is_binary, is_folder, is_symlink, name, owner,
    parent, path, permissions, size,
};
use super::node;
use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{FileRef, Object, ObjectTable};
use crate::protocol::{
    Failure, FileAllowImport, FileChange, FileChmod, FileCopy, FileDate, FileDelete, FileEntries,
    FileFailure, FileGetContent, FileGetFiles, FileGetFolders, FileGroup, FileHasPermission,
    FileIsBinary, FileIsFolder, FileIsSymlink, FileMove, FileName, FileOwner, FileParent, FilePath,
    FilePermissions, FileRename, FileSetContent, FileSetGroup, FileSetOwner, FileSize, FileSymlink,
    GameServer, ObjectId, Outcome,
};

enum EntryResult {
    Null,
    Nodes(Vec<Node>),
    Failure(Failure),
}

/// Lists the subfolders or files of folder object `id` as file objects.
async fn file_entries(
    world: &World,
    objects: &mut ObjectTable,
    id: ObjectId,
    folders: bool,
) -> Result<FileEntries> {
    let file = objects.file(id)?.clone();

    Ok(match entries(world, &file, folders).await? {
        EntryResult::Nodes(entries) => FileEntries::Entries(
            entries
                .into_iter()
                .map(|node| add_file(objects, &file, node))
                .collect(),
        ),
        EntryResult::Null => FileEntries::Null,
        EntryResult::Failure(failure) => FileEntries::Failure(failure),
    })
}

/// Points file object `id` at `node` after a copy, move, or rename.
fn retarget(objects: &mut ObjectTable, id: ObjectId, node: Node) -> Result<()> {
    let file = objects.file_mut(id)?;

    file.node = node.id;
    file.path = node.path;

    Ok(())
}

/// Registers `node` as a file object with `file`'s access identity.
fn add_file(objects: &mut ObjectTable, file: &FileRef, node: Node) -> ObjectId {
    objects.add(Object::File(FileRef {
        access: file.access.clone(),
        node: node.id,
        path: node.path,
    }))
}

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(channel, async |world, objects, message: FilePath, _| {
        path(world, objects.file(message.file)?, message.original).await
    });
    connection.read(channel, async |world, objects, message: FileParent, _| {
        let file = objects.file(message.file)?.clone();

        Ok(parent(world, &file)
            .await?
            .map(|parent| add_file(objects, &file, parent)))
    });
    connection.read(channel, async |world, objects, message: FileName, _| {
        name(world, objects.file(message.file)?).await
    });
    connection.read(channel, async |world, objects, message: FileIsFolder, _| {
        is_folder(world, objects.file(message.file)?).await
    });
    connection.read(
        channel,
        async |world, objects, message: FileIsSymlink, _| {
            is_symlink(world, objects.file(message.file)?).await
        },
    );
    connection.read(channel, async |world, objects, message: FileSize, _| {
        size(world, objects.file(message.file)?).await
    });
    connection.read(channel, async |world, objects, message: FileDate, _| {
        date(world, objects.file(message.file)?).await
    });
    connection.read(
        channel,
        async |world, objects, message: FileGetFolders, _| {
            file_entries(world, objects, message.file, true).await
        },
    );
    connection.read(channel, async |world, objects, message: FileGetFiles, _| {
        file_entries(world, objects, message.file, false).await
    });
    connection.read(
        channel,
        async |world, objects, message: FileGetContent, _| {
            content(world, objects.file(message.file)?).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileSetContent<'static>, _| {
            set_content(world, objects.file(message.file)?, &message.content).await
        },
    );
    connection.read(channel, async |world, objects, message: FileIsBinary, _| {
        is_binary(world, objects.file(message.file)?).await
    });
    connection.read(
        channel,
        async |world, objects, message: FileAllowImport, _| {
            allow_import(world, objects.file(message.file)?).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileCopy<'static>, _| {
            let (outcome, replacement) = copy(
                world,
                objects.file(message.file)?,
                &message.folder,
                &message.name,
            )
            .await?;

            if matches!(
                &outcome,
                Outcome::True | Outcome::Failure(Failure::File(FileFailure::PartialCopy))
            ) && let Some(replacement) = replacement
            {
                retarget(objects, message.file, replacement)?;
            }

            Ok(outcome)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileMove<'static>, _| {
            let (outcome, new_node) = r#move(
                world,
                objects.file(message.file)?,
                &message.folder,
                &message.name,
            )
            .await?;

            if outcome == Outcome::True
                && let Some(new_node) = new_node
            {
                retarget(objects, message.file, new_node)?;
            }

            Ok(outcome)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileSymlink<'static>, _| {
            symlink(
                world,
                objects.file(message.file)?,
                &message.folder,
                &message.name,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileRename<'static>, _| {
            let (result, new_node) =
                rename(world, objects.file(message.file)?, &message.name).await?;

            if let Some(new_node) = new_node {
                retarget(objects, message.file, new_node)?;
            }

            Ok(result)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileDelete, caller| {
            delete(
                world,
                objects.file(message.file)?,
                u64::from(caller.program),
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileChmod<'static>, _| {
            chmod(
                world,
                objects.file(message.file)?,
                &message.change,
                message.recursive,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: FilePermissions, _| {
            permissions(world, objects.file(message.file)?).await
        },
    );
    connection.read(channel, async |world, objects, message: FileOwner, _| {
        owner(world, objects.file(message.file)?).await
    });
    connection.read(channel, async |world, objects, message: FileGroup, _| {
        group(world, objects.file(message.file)?).await
    });
    connection.change(
        channel,
        async |world, objects, message: FileSetOwner<'static>, _| {
            set_owner(
                world,
                objects.file(message.file)?,
                Some(&message.owner),
                None,
                message.recursive,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: FileSetGroup<'static>, _| {
            set_owner(
                world,
                objects.file(message.file)?,
                None,
                Some(&message.group),
                message.recursive,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: FileHasPermission<'static>, _| {
            has_permission(world, objects.file(message.file)?, &message.permission).await
        },
    );
}

/// Looks up the node and optionally resolves its final symlink.
async fn resolve(world: &World, file: &FileRef, follow: bool) -> Result<Option<Node>> {
    let Some(node) = node(world, file).await? else {
        return Ok(None);
    };

    if follow && node.is_symlink() {
        Ok(world.entry(node.computer, &node.path, true).await?)
    } else {
        Ok(Some(node))
    }
}

/// Resolves the node only if the shell user exists and has path traversal access.
async fn accessible_resolve(world: &World, file: &FileRef, follow: bool) -> Result<Option<Node>> {
    world
        .entry_for_user(file.access.computer, &file.path, follow, &file.access.user)
        .await
        .map_err(Into::into)
}

/// Wraps file `failure` as an operation outcome.
const fn file_failure(failure: FileFailure) -> Outcome {
    Outcome::Failure(Failure::File(failure))
}

/// Converts file `failure` into a change response.
const fn file_change(failure: FileFailure) -> FileChange {
    FileChange::Failure(Failure::File(failure))
}
