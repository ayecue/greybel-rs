//! Inspection, patching, testing, and payload generation for debug libraries.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{DebugPayload, PartialObject as WorldPartialObject};

use crate::connection::Connection;
use crate::connection::objects::{Access, FileRef, LibraryRef, MetaLibRef, Object};
use crate::protocol::{
    DebugLibraryApplyPatch, DebugLibraryPayload, DebugLibraryScanDebuglib, DebugLibraryUnitTesting,
    DebugScan, Failure, GameServer, PartialObject, ScriptFailure, UnitTestReport, ZeroDayPayload,
};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: DebugLibraryScanDebuglib, _| {
            let library = objects.debug_library(message.library)?;

            Ok(world
                .scan_debug_library(library.access.computer, &library.path)
                .await?
                .map(DebugScan::from)
                .map_err(|failure| ScriptFailure::Refused(Failure::Debug(failure.into())))?)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: DebugLibraryApplyPatch<'static>, _| {
            let library = objects.debug_library(message.library)?;

            world
                .patch_debug_library(library.access.computer, &library.path, &message.path)
                .await?
                .map(|()| true)
                .map_err(|failure| ScriptFailure::Refused(Failure::Debug(failure.into())))
                .map_err(Into::into)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: DebugLibraryUnitTesting<'static>, caller| {
            let library = objects.debug_library(message.library)?;
            let user = &objects.session(caller.session)?.user;

            Ok(world
                .run_debug_library_tests(
                    library.access.computer,
                    &library.path,
                    user,
                    &message.error_lines,
                )
                .await?
                .map(UnitTestReport::from)
                .map_err(|failure| ScriptFailure::Refused(Failure::Debug(failure.into())))?)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: DebugLibraryPayload<'static>, _| {
            let library = objects.debug_library(message.library)?;
            let origin = library.access.clone();
            let access = |computer, user| Access {
                computer,
                user,
                ..origin.clone()
            };
            let payload = world
                .debug_library_payload(
                    library.access.computer,
                    &library.path,
                    &message.mem_zone,
                    &message.path,
                )
                .await?;

            Ok(match payload {
                DebugPayload::Failure(failure) => {
                    ZeroDayPayload::Failure(Failure::Debug(failure.into()))
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
                                computer: objects.add(Object::Computer(access(computer, user))),
                                methods,
                            },
                            WorldPartialObject::File {
                                computer,
                                node,
                                path,
                                user,
                                methods,
                            } => PartialObject::File {
                                file: objects.add(Object::File(FileRef {
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
                                library: objects.add(Object::MetaLib(MetaLibRef::File(
                                    LibraryRef {
                                        access: access(computer, user),
                                        path,
                                        origin: computer,
                                    },
                                ))),
                                methods,
                            },
                        })
                        .collect(),
                ),
            })
        },
    );
}
