//! `metaLib` operations on computer libraries and router `kernel_router.so` files.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{DebugLibraryPatch, ExploitOutcome, Library, World};

use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{Access, FileRef, MetaLibRef, Object};
use crate::protocol::{
    Exploit, ExploitObject, Failure, GameServer, MetaLibDebugTools, MetaLibIsPatched,
    MetaLibLibName, MetaLibOverflow, MetaLibVersion, ObjectOutcome, PatchState, ScriptFailure,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: MetaLibLibName, _| {
            lib_name(world, objects.meta_lib(message.library)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: MetaLibVersion, _| {
            version(world, objects.meta_lib(message.library)?).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaLibDebugTools<'static>, _| {
            let MetaLibRef::File(library) = objects.meta_lib(message.library)?.clone() else {
                return Ok(ObjectOutcome::Null);
            };

            Ok(
                match world.login_debug(&message.user, &message.password).await? {
                    Ok(()) => ObjectOutcome::Object(objects.add(Object::DebugLibrary(library))),
                    Err(failure) => ObjectOutcome::Failure(Failure::Debug(failure.into())),
                },
            )
        },
    );
    connection.read(
        channel,
        async |world, objects, message: MetaLibIsPatched, _| {
            let MetaLibRef::File(library) = objects.meta_lib(message.library)? else {
                return Ok(PatchState::Null);
            };

            Ok(
                match world
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
    connection.change(
        channel,
        async |world, objects, message: MetaLibOverflow<'static>, caller| {
            let session_computer = objects.session(caller.session)?.computer;
            let args = message
                .args
                .as_deref()
                .ok_or(ScriptFailure::InvalidArguments)?;
            let meta_lib = objects.meta_lib(message.library)?.clone();
            // `HelperValMetaLib.prevBounceIP`, and the computer whose metaxploit opened the library.
            let (prev_ip, origin, bounces) = match &meta_lib {
                MetaLibRef::File(file) => (
                    file.access.prev_ip.clone(),
                    file.origin,
                    file.access.bounces,
                ),
                MetaLibRef::Kernel(kernel) => (
                    world.public_ip(kernel.origin).await?,
                    kernel.origin,
                    kernel.bounces,
                ),
            };
            let library_name = super::meta_lib::lib_name(world, &meta_lib).await?;
            let report = match &meta_lib {
                MetaLibRef::File(file) => {
                    world
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
                        .log_overflow_shell(
                            library_name.as_deref().unwrap_or_default(),
                            &prev_ip,
                            origin,
                            computer,
                        )
                        .await?;

                    ExploitObject::Shell(objects.add(Object::Shell(Access {
                        computer,
                        user,
                        ftp: false,
                        prev_ip: origin_ip,
                        bounces: bounces + 1,
                    })))
                }
                ExploitOutcome::Computer { computer, user } => {
                    ExploitObject::Computer(objects.add(Object::Computer(Access {
                        computer,
                        user,
                        ftp: false,
                        prev_ip,
                        bounces: bounces + 1,
                    })))
                }
                ExploitOutcome::File {
                    computer,
                    node,
                    path,
                    user,
                } => ExploitObject::File(objects.add(Object::File(FileRef {
                    access: Access {
                        computer,
                        user,
                        ftp: false,
                        prev_ip,
                        bounces,
                    },
                    node,
                    path,
                }))),
            };

            Ok(Exploit {
                log: report.log.into(),
                object,
            })
        },
    );
}

/// Applies `read` to either library kind referenced by `library`.
///
/// # Errors
///
/// Returns errors from database reads.
pub(crate) async fn library<T>(
    world: &World,
    library: &MetaLibRef,
    read: impl FnOnce(&Library) -> T,
) -> Result<Option<T>> {
    match library {
        MetaLibRef::File(file) => Ok(world
            .file_library(file.access.computer, &file.path, false)
            .await?
            .map(|library| read(&library))),
        MetaLibRef::Kernel(kernel) => Ok(world
            .kernel_library(kernel.router)
            .await?
            .map(|library| read(&library))),
    }
}

/// Looks up the library filename.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn lib_name(world: &World, meta_lib: &MetaLibRef) -> Result<Option<String>> {
    library(world, meta_lib, |library| library.kind.file_name()).await
}

/// Looks up the library version.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn version(world: &World, meta_lib: &MetaLibRef) -> Result<Option<String>> {
    library(world, meta_lib, |library| library.version.clone()).await
}
