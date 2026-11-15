//! `metaLib` operations on computer libraries and router `kernel_router.so` files.

use greyhack_world_core::{
    Access, AccessHandle, AccessKind, FileHandle, FileRef, ObjectTableExt as _,
};
use greyhack_world_router::RouteTable;

use crate::objects::{DebugLibraryHandle, MetaLibRef, ObjectTableExt};
use crate::routes::protocol::{
    Exploit, ExploitObject, Failure, IntoProtocol as _, MetaLibDebugTools, MetaLibIsPatched,
    MetaLibLibName, MetaLibOverflow, MetaLibVersion, ObjectOutcome, PatchState, PrintLine,
    ScriptFailure,
};
use crate::routes::{OperationError, Result};
use crate::{DebugLibraryPatch, ExploitOutcome, Library};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaLibLibName| {
            let world = &*ctx.modules;

            lib_name(
                world,
                &ctx.trees.objects(call.tree).meta_lib(message.library)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaLibVersion| {
            let world = &*ctx.modules;

            version(
                world,
                &ctx.trees.objects(call.tree).meta_lib(message.library)?,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaLibDebugTools<'static>| {
            let world = &*ctx.modules;
            let MetaLibRef::File(library) = ctx
                .trees
                .objects(call.tree)
                .meta_lib(message.library)?
                .clone()
            else {
                return Ok(ObjectOutcome::Null);
            };

            Ok(
                match world
                    .module::<crate::LibraryModule>()
                    .debug
                    .login_debug(&message.user, &message.password)
                    .await?
                {
                    Ok(()) => ObjectOutcome::Object(
                        ctx.trees
                            .objects(call.tree)
                            .insert(DebugLibraryHandle(library)),
                    ),
                    Err(failure) => ObjectOutcome::Failure(Failure::Debug(failure.into_protocol())),
                },
            )
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaLibIsPatched| {
            let world = &*ctx.modules;
            let MetaLibRef::File(library) =
                &ctx.trees.objects(call.tree).meta_lib(message.library)?
            else {
                return Ok(PatchState::Null);
            };

            Ok(
                match world
                    .module::<crate::LibraryModule>()
                    .debug
                    .debug_library_patch(library.access.computer, &library.path, message.date)
                    .await?
                {
                    DebugLibraryPatch::NoPatch => PatchState::Unpatched,
                    DebugLibraryPatch::PatchPresent => PatchState::Patched,
                    DebugLibraryPatch::AutoPatchAt(date) => PatchState::AutoPatchAt(date),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaLibOverflow<'static>| {
            let world = &*ctx.modules;
            let session_computer = ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .computer;
            let args = message
                .args
                .as_deref()
                .ok_or(OperationError::Script(ScriptFailure::InvalidArguments))?;
            let meta_lib = ctx
                .trees
                .objects(call.tree)
                .meta_lib(message.library)?
                .clone();
            // `HelperValMetaLib.prevBounceIP`, and the computer whose metaxploit opened the library.
            let (prev_ip, origin, bounces) = match &meta_lib {
                MetaLibRef::File(file) => (
                    file.access.prev_ip.clone(),
                    file.origin,
                    file.access.bounces,
                ),
                MetaLibRef::Kernel(kernel) => (
                    world
                        .module::<greyhack_world_core::CoreModule>()
                        .networks
                        .topology
                        .public_ip(kernel.origin)
                        .await?,
                    kernel.origin,
                    kernel.bounces,
                ),
            };
            let library_name = super::meta_lib::lib_name(world, &meta_lib).await?;
            let report = match &meta_lib {
                MetaLibRef::File(file) => {
                    world
                        .module::<crate::LibraryModule>()
                        .overflows
                        .overflow_file(
                            file.access.computer,
                            &file.path,
                            &message.address,
                            &message.unsec_value,
                            args,
                        )
                        .await?
                }
                MetaLibRef::Kernel(kernel) => {
                    world
                        .module::<crate::LibraryModule>()
                        .overflows
                        .overflow_router(
                            kernel.router,
                            &message.address,
                            &message.unsec_value,
                            session_computer,
                            (&kernel.address, kernel.origin),
                            args,
                        )
                        .await?
                }
            };

            let object = match report.outcome {
                ExploitOutcome::Nothing => ExploitObject::Null,
                ExploitOutcome::DisabledNetwork => ExploitObject::DisabledNetwork,
                ExploitOutcome::Success => ExploitObject::True,
                ExploitOutcome::Failure => ExploitObject::False,
                ExploitOutcome::Shell { computer, user } => {
                    // The shell's previous address is the metaxploit computer's public address.
                    let origin_ip = world
                        .module::<crate::LibraryModule>()
                        .overflows
                        .log_overflow_shell(
                            library_name.as_deref().unwrap_or_default(),
                            &prev_ip,
                            origin,
                            computer,
                        )
                        .await?;

                    ExploitObject::Shell(ctx.trees.objects(call.tree).insert(AccessHandle {
                        access: Access {
                            computer,
                            user,
                            ftp: false,
                            prev_ip: origin_ip,
                            bounces: bounces + 1,
                        },
                        kind: AccessKind::Shell,
                    }))
                }
                ExploitOutcome::Computer { computer, user } => {
                    ExploitObject::Computer(ctx.trees.objects(call.tree).insert(AccessHandle {
                        access: Access {
                            computer,
                            user,
                            ftp: false,
                            prev_ip,
                            bounces: bounces + 1,
                        },
                        kind: AccessKind::Computer,
                    }))
                }
                ExploitOutcome::File {
                    computer,
                    node,
                    path,
                    user,
                } => {
                    ExploitObject::File(ctx.trees.objects(call.tree).insert(FileHandle(FileRef {
                        access: Access {
                            computer,
                            user,
                            ftp: false,
                            prev_ip,
                            bounces,
                        },
                        node,
                        path,
                    })))
                }
            };

            if report.log != crate::ExploitLog::Nothing {
                ctx.print(call, PrintLine::ExploitLog(report.log.into_protocol()));
            }

            Ok(Exploit { object })
        },
    );
}

/// Applies `read` to either library kind referenced by `library`.
///
/// # Errors
///
/// Returns errors from database reads.
pub(crate) async fn library<T>(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    library: &MetaLibRef,
    read: impl FnOnce(&Library) -> T,
) -> Result<Option<T>> {
    match library {
        MetaLibRef::File(file) => Ok(world
            .module::<crate::LibraryModule>()
            .generator
            .full_file_library(file.access.computer, &file.path, false)
            .await?
            .map(|library| read(&library))),
        MetaLibRef::Kernel(kernel) => {
            let file = world
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .routers
                .kernel_file(kernel.router)
                .await?;
            let library = match &file {
                Some(file) => {
                    world
                        .module::<crate::LibraryModule>()
                        .generator
                        .full_library_of(file)
                        .await?
                }
                None => None,
            };

            Ok(library.map(|library| read(&library)))
        }
    }
}

/// Looks up the library filename.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn lib_name(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    meta_lib: &MetaLibRef,
) -> Result<Option<String>> {
    library(world, meta_lib, |library| library.kind.file_name()).await
}

/// Looks up the library version.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn version(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    meta_lib: &MetaLibRef,
) -> Result<Option<String>> {
    library(world, meta_lib, |library| library.version.clone()).await
}
