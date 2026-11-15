//! File-object operations (`FileIntrinsics`) with current-path resolution per call.

mod changes;
mod content;
mod ownership;
mod properties;

use greyhack_world_programs::ObjectTableHandle;
use greyhack_world_programs::protocol::{
    Failure, FileAllowImport, FileChange, FileChmod, FileCopy, FileDate, FileDelete, FileEntries,
    FileFailure, FileGetContent, FileGetFiles, FileGetFolders, FileGroup, FileHasPermission,
    FileIsBinary, FileIsFolder, FileIsSymlink, FileMove, FileName, FileOwner, FileParent, FilePath,
    FilePermissions, FileRename, FileSetContent, FileSetGroup, FileSetOwner, FileSize, FileSymlink,
    ObjectId, Outcome,
};
use greyhack_world_router::{RouteContext, RouteTable};

use self::changes::{copy, delete, r#move, rename, symlink};
use self::content::{content, entries, set_content};
use self::ownership::{chmod, set_owner};
use self::properties::{
    allow_import, date, group, has_permission, is_binary, is_folder, is_symlink, name, owner,
    parent, path, permissions, size,
};
use super::node;
use crate::objects::{FileHandle, FileRef, ObjectTableExt};
use crate::routes::Result;
use crate::{ComputerId, Node};

enum EntryResult {
    Null,
    Nodes(Vec<Node>),
    Failure(Failure),
}

/// Lists the subfolders or files of folder object `id` as file objects.
async fn file_entries(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    objects: &mut ObjectTableHandle<ComputerId>,
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
fn retarget(objects: &mut ObjectTableHandle<ComputerId>, id: ObjectId, node: Node) -> Result<()> {
    let mut file = objects.file_mut(id)?;

    file.node = node.id;
    file.path = node.path;
    objects.set_file(id, file)?;

    Ok(())
}

/// Registers `node` as a file object with `file`'s access identity.
fn add_file(objects: &mut ObjectTableHandle<ComputerId>, file: &FileRef, node: Node) -> ObjectId {
    objects.insert(FileHandle(FileRef {
        access: file.access.clone(),
        node: node.id,
        path: node.path,
    }))
}

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = crate::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FilePath| {
            let world = &*ctx.modules;

            path(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                message.original,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileParent| {
            let world = &*ctx.modules;
            let file = &ctx.trees.objects(call.tree).file(message.file)?.clone();

            Ok(parent(world, file)
                .await?
                .map(|parent| add_file(&mut ctx.trees.objects(call.tree), file, parent)))
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileName| {
            let world = &*ctx.modules;

            name(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileIsFolder| {
            let world = &*ctx.modules;

            is_folder(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileIsSymlink| {
            let world = &*ctx.modules;

            is_symlink(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileSize| {
            let world = &*ctx.modules;

            size(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileDate| {
            let world = &*ctx.modules;

            date(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileGetFolders| {
            let world = &*ctx.modules;

            file_entries(world, &mut ctx.trees.objects(call.tree), message.file, true).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileGetFiles| {
            let world = &*ctx.modules;

            file_entries(
                world,
                &mut ctx.trees.objects(call.tree),
                message.file,
                false,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileGetContent| {
            let world = &*ctx.modules;

            content(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileSetContent<'static>| {
            let world = &*ctx.modules;

            set_content(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.content,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileIsBinary| {
            let world = &*ctx.modules;

            is_binary(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileAllowImport| {
            let world = &*ctx.modules;

            allow_import(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileCopy<'static>| {
            let world = &*ctx.modules;
            let (outcome, replacement) = copy(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.folder,
                &message.name,
            )
            .await?;

            if matches!(
                &outcome,
                Outcome::True | Outcome::Failure(Failure::File(FileFailure::PartialCopy))
            ) && let Some(replacement) = replacement
            {
                retarget(&mut ctx.trees.objects(call.tree), message.file, replacement)?;
            }

            Ok(outcome)
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileMove<'static>| {
            let world = &*ctx.modules;
            let (outcome, new_node) = r#move(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.folder,
                &message.name,
            )
            .await?;

            if outcome == Outcome::True
                && let Some(new_node) = new_node
            {
                retarget(&mut ctx.trees.objects(call.tree), message.file, new_node)?;
            }

            Ok(outcome)
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileSymlink<'static>| {
            let world = &*ctx.modules;

            symlink(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.folder,
                &message.name,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileRename<'static>| {
            let world = &*ctx.modules;
            let (result, new_node) = rename(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.name,
            )
            .await?;

            if let Some(new_node) = new_node {
                retarget(&mut ctx.trees.objects(call.tree), message.file, new_node)?;
            }

            Ok(result)
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileDelete| {
            let world = &*ctx.modules;
            let last_shell = ctx.last_shell(
                call,
                world.module::<crate::CoreModule>().world.clone().player(),
            );

            delete(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                u64::from(call.program.id),
                last_shell,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileChmod<'static>| {
            let world = &*ctx.modules;

            chmod(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.change,
                message.recursive,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FilePermissions| {
            let world = &*ctx.modules;

            permissions(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileOwner| {
            let world = &*ctx.modules;

            owner(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileGroup| {
            let world = &*ctx.modules;

            group(world, &ctx.trees.objects(call.tree).file(message.file)?).await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileSetOwner<'static>| {
            let world = &*ctx.modules;

            set_owner(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                Some(&message.owner),
                None,
                message.recursive,
            )
            .await
        },
    );
    routes.change::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileSetGroup<'static>| {
            let world = &*ctx.modules;

            set_owner(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                None,
                Some(&message.group),
                message.recursive,
            )
            .await
        },
    );
    routes.read::<_, _, W, crate::WorldError>(
        async |ctx: &RouteContext<'_, W>, call, message: FileHasPermission<'static>| {
            let world = &*ctx.modules;

            has_permission(
                world,
                &ctx.trees.objects(call.tree).file(message.file)?,
                &message.permission,
            )
            .await
        },
    );
}

/// Looks up the node and optionally resolves its final symlink.
async fn resolve(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    follow: bool,
) -> Result<Option<Node>> {
    let Some(node) = node(world, file).await? else {
        return Ok(None);
    };

    if follow && node.is_symlink() {
        Ok(world
            .module::<crate::CoreModule>()
            .files
            .lookup
            .entry(node.computer, &node.path, true)
            .await?)
    } else {
        Ok(Some(node))
    }
}

/// Resolves the node only if the shell user exists and has path traversal access.
async fn accessible_resolve(
    world: &impl greyhack_world_router::WorldContext<Computer = crate::ComputerId>,
    file: &FileRef,
    follow: bool,
) -> Result<Option<Node>> {
    world
        .module::<crate::CoreModule>()
        .files
        .lookup
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
