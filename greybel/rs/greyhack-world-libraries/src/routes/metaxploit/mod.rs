//! Library scans, net sessions, reverse shells, and packet sniffing (`MetaExploitIntrinsics`).

use greyhack_world_core::routes::require_connection;
use greyhack_world_core::{Access, AccessHandle, AccessKind, ReverseShellHandle};
use greyhack_world_router::RouteTable;

use crate::RshellServerFailure;
use crate::objects::{LibraryRef, MetaLibHandle, MetaLibRef, NetSessionHandle, ObjectTableExt};
use crate::routes::protocol::{
    Failure, IntoProtocol as _, MetaxploitFailure, MetaxploitLoad, MetaxploitNetUse,
    MetaxploitRshellClient, MetaxploitRshellRemove, MetaxploitRshellServer, MetaxploitScan,
    MetaxploitScanAddress, NetUseTarget, PrintLine, ReverseShellClient, RshellServerResponse,
};

mod rshell;
mod scan;
mod session;
use self::rshell::{rshell_client, rshell_remove};
use self::scan::{scan, scan_address, scan_address_without_metaxploit};
use self::session::net_use_with_optional_access;

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    register_metaxploit_scans::<W>(routes);
    register_metaxploit_sessions::<W>(routes);
    register_metaxploit_server::<W>(routes);
}

fn register_metaxploit_scans<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitLoad<'static>| {
            let world = &*ctx.modules;
            let access = ctx
                .trees
                .objects(call.tree)
                .metaxploit(message.metaxploit)?
                .access
                .clone();
            let path = world
                .module::<crate::LibraryModule>()
                .metaxploit
                .library_file_path(access.computer, &message.path)
                .await?;

            Ok(path.map(|path| {
                ctx.trees
                    .objects(call.tree)
                    .insert(MetaLibHandle(MetaLibRef::File(LibraryRef {
                        origin: access.computer,
                        access,
                        path,
                    })))
            }))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitScan| {
            let version = &ctx
                .trees
                .objects(call.tree)
                .metaxploit(message.metaxploit)?
                .version;

            scan(
                ctx,
                call,
                &ctx.trees.objects(call.tree).meta_lib(message.library)?,
                version,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitScanAddress<'static>| {
            let world = &*ctx.modules;
            let Some(metaxploit) = message.metaxploit else {
                return scan_address_without_metaxploit(
                    world,
                    &ctx.trees.objects(call.tree).meta_lib(message.library)?,
                    &message.address,
                )
                .await;
            };
            let version = &ctx.trees.objects(call.tree).metaxploit(metaxploit)?.version;

            scan_address(
                world,
                &ctx.trees.objects(call.tree).meta_lib(message.library)?,
                version,
                &message.address,
            )
            .await
        },
    );
}

fn register_metaxploit_sessions<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitNetUse<'static>| {
            let world = &*ctx.modules;
            let access = match message.metaxploit {
                Some(metaxploit) => {
                    Some(&ctx.trees.objects(call.tree).metaxploit(metaxploit)?.access)
                }
                None => None,
            };
            let terminal_computers = &ctx.terminal_computers(call);

            Ok(
                match net_use_with_optional_access(
                    world,
                    access,
                    terminal_computers,
                    &message.address,
                    message.port,
                )
                .await?
                {
                    Ok(Some(session)) => NetUseTarget::Session(
                        ctx.trees
                            .objects(call.tree)
                            .insert(NetSessionHandle(session)),
                    ),
                    Ok(None) => NetUseTarget::Null,
                    Err(failure) => {
                        ctx.print(
                            call,
                            PrintLine::NetUseFailure {
                                failure: failure.into_protocol(),
                                address: message.address.to_string(),
                            },
                        );

                        NetUseTarget::Null
                    }
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitRshellClient<'static>| {
            let world = &*ctx.modules;

            rshell_client(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .metaxploit(message.metaxploit)?
                    .access,
                &ctx.terminal_computers(call),
                &message.address,
                message.port,
                &message.process_name,
            )
            .await
        },
    );
}

fn register_metaxploit_server<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitRshellServer| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .metaxploit(message.metaxploit)?
                .access;
            let terminal_computers = &ctx.terminal_computers(call);

            require_connection(world, terminal_computers).await?;

            let origin = world
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .topology
                .public_ip(access.computer)
                .await?;

            Ok(
                match world
                    .module::<crate::LibraryModule>()
                    .metaxploit
                    .rshell_clients(access.computer)
                    .await?
                {
                    None => RshellServerResponse::Null,
                    Some(Err(failure)) => {
                        RshellServerResponse::Failure(Failure::Metaxploit(match failure {
                            RshellServerFailure::NoService => MetaxploitFailure::RshellNotRunning,
                            RshellServerFailure::ForwardMissing => {
                                MetaxploitFailure::RshellForwardMissing
                            }
                        }))
                    }
                    Some(Ok(clients)) => RshellServerResponse::Clients(
                        clients
                            .into_iter()
                            .map(|client| {
                                let access = Access {
                                    computer: client.computer,
                                    user: client.user,
                                    ftp: false,
                                    prev_ip: origin.clone(),
                                    bounces: 1,
                                };

                                ReverseShellClient {
                                    computer: ctx.trees.objects(call.tree).insert(AccessHandle {
                                        access: access.clone(),
                                        kind: AccessKind::Computer,
                                    }),
                                    shell: ctx
                                        .trees
                                        .objects(call.tree)
                                        .insert(ReverseShellHandle(access, client.pid)),
                                }
                            })
                            .collect(),
                    ),
                },
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: MetaxploitRshellRemove| {
            rshell_remove(
                &*ctx.modules,
                &ctx.trees.objects(call.tree),
                message.metaxploit,
                message.shells,
                &ctx.terminal_computers(call),
            )
            .await
        },
    );
}
