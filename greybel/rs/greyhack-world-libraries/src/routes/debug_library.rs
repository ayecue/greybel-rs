//! Inspection, patching, testing, and payload generation for debug libraries.

use greyhack_world_core::{
    Access, AccessHandle, AccessKind, FileHandle, FileRef, ObjectTableExt as _,
};
use greyhack_world_router::RouteTable;

use crate::objects::{LibraryRef, MetaLibHandle, MetaLibRef, ObjectTableExt};
use crate::routes::OperationError;
use crate::routes::protocol::{
    DebugFailure, DebugLibraryApplyPatch, DebugLibraryPayload, DebugLibraryScanDebuglib,
    DebugLibraryUnitTesting, Failure, IntoProtocol, PartialObject, PrintLine, ScriptFailure,
    ZeroDayPayload,
};
use crate::{DebugPayload, PartialObject as WorldPartialObject};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    register_debug_library_tests::<W>(routes);
    register_debug_library_payload::<W>(routes);
}

fn register_debug_library_tests<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: DebugLibraryScanDebuglib| {
            let world = &*ctx.modules;
            let library = &ctx
                .trees
                .objects(call.tree)
                .debug_library(message.library)?;

            world
                .module::<crate::LibraryModule>()
                .debug
                .scan_debug_library(library.access.computer, &library.path)
                .await?
                .map(IntoProtocol::into_protocol)
                .map_err(|failure| {
                    OperationError::Script(ScriptFailure::Refused(Failure::Debug(
                        failure.into_protocol(),
                    )))
                })
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: DebugLibraryApplyPatch<'static>| {
            let world = &*ctx.modules;
            let library = &ctx
                .trees
                .objects(call.tree)
                .debug_library(message.library)?;

            world
                .module::<crate::LibraryModule>()
                .debug
                .patch_debug_library(library.access.computer, &library.path, &message.path)
                .await?
                .map(|()| true)
                .map_err(|failure| {
                    OperationError::Script(ScriptFailure::Refused(Failure::Debug(
                        failure.into_protocol(),
                    )))
                })
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: DebugLibraryUnitTesting<'static>| {
            let world = &*ctx.modules;
            let library = &ctx
                .trees
                .objects(call.tree)
                .debug_library(message.library)?;
            let user = &ctx
                .trees
                .objects(call.tree)
                .session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?
                .user;

            world
                .module::<crate::LibraryModule>()
                .debug
                .run_debug_library_tests(
                    library.access.computer,
                    &library.path,
                    user,
                    &message.error_lines,
                )
                .await?
                .map(IntoProtocol::into_protocol)
                .map_err(|failure| {
                    OperationError::Script(ScriptFailure::Refused(Failure::Debug(
                        failure.into_protocol(),
                    )))
                })
        },
    );
}

fn register_debug_library_payload<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: DebugLibraryPayload<'static>| {
            let world = &*ctx.modules;
            let library = &ctx
                .trees
                .objects(call.tree)
                .debug_library(message.library)?;
            let origin = library.access.clone();
            let access = |computer, user| Access {
                computer,
                user,
                ..origin.clone()
            };
            let payload = world
                .module::<crate::LibraryModule>()
                .debug
                .debug_library_payload(
                    library.access.computer,
                    &library.path,
                    &message.mem_zone,
                    &message.path,
                )
                .await?;

            Ok(match payload {
                DebugPayload::Failure(failure) => {
                    let failure = failure.into_protocol();

                    if matches!(failure, DebugFailure::OutsideLib) {
                        ctx.print(call, PrintLine::PayloadFailure(Failure::Debug(failure)));

                        ZeroDayPayload::Null
                    } else {
                        ZeroDayPayload::Failure(Failure::Debug(failure))
                    }
                }
                DebugPayload::Objects(partials) => ZeroDayPayload::Objects(
                    partials
                        .into_iter()
                        .map(|partial| match partial {
                            WorldPartialObject::Computer {
                                computer,
                                user,
                                methods,
                            } => PartialObject::Computer {
                                computer: ctx.trees.objects(call.tree).insert(AccessHandle {
                                    access: access(computer, user),
                                    kind: AccessKind::Computer,
                                }),
                                methods,
                            },
                            WorldPartialObject::File {
                                computer,
                                node,
                                path,
                                user,
                                methods,
                            } => PartialObject::File {
                                file: ctx.trees.objects(call.tree).insert(FileHandle(FileRef {
                                    access: access(computer, user),
                                    node,
                                    path,
                                })),
                                methods,
                            },
                            WorldPartialObject::Library {
                                computer,
                                path,
                                user,
                                methods,
                            } => PartialObject::MetaLib {
                                library: ctx.trees.objects(call.tree).insert(MetaLibHandle(
                                    MetaLibRef::File(LibraryRef {
                                        access: access(computer, user),
                                        path,
                                        origin: computer,
                                    }),
                                )),
                                methods,
                            },
                        })
                        .collect(),
                ),
            })
        },
    );
}
