//! Library scans, net sessions, reverse shells, and packet sniffing (`MetaExploitIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{RshellServerFailure, WorkResult};

use super::require_connection;
use crate::OperationError;
use crate::connection::Connection;
use crate::connection::objects::{Access, LibraryRef, MetaLibRef, Object};
use crate::protocol::{
    Failure, GameServer, MetaxploitFailure, MetaxploitLoad, MetaxploitNetUse,
    MetaxploitRshellClient, MetaxploitRshellServer, MetaxploitScan, MetaxploitScanAddress,
    MetaxploitSniffer, NetUseTarget, ReverseShellClient, RshellServerResponse, ScriptFailure,
};

mod rshell;
mod scan;
mod session;
mod sniffer;
use self::rshell::rshell_client;
use self::scan::{scan, scan_address, scan_address_without_metaxploit};
use self::session::net_use_with_optional_access;
use self::sniffer::capture;

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: MetaxploitLoad<'static>, _| {
            let access = objects.metaxploit(message.metaxploit)?.access.clone();
            let path = world
                .library_file_path(access.computer, &message.path)
                .await?;

            Ok(path.map(|path| {
                objects.add(Object::MetaLib(MetaLibRef::File(LibraryRef {
                    origin: access.computer,
                    access,
                    path,
                })))
            }))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: MetaxploitScan, _| {
            let version = &objects.metaxploit(message.metaxploit)?.version;

            scan(world, objects.meta_lib(message.library)?, version).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: MetaxploitScanAddress<'static>, _| {
            let Some(metaxploit) = message.metaxploit else {
                return scan_address_without_metaxploit(
                    world,
                    objects.meta_lib(message.library)?,
                    &message.address,
                )
                .await;
            };
            let version = &objects.metaxploit(metaxploit)?.version;

            scan_address(
                world,
                objects.meta_lib(message.library)?,
                version,
                &message.address,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaxploitNetUse<'static>, caller| {
            let access = match message.metaxploit {
                Some(metaxploit) => Some(&objects.metaxploit(metaxploit)?.access),
                None => None,
            };
            let terminal_computers = &caller.terminal_computers;

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
                    Ok(Some(session)) => {
                        NetUseTarget::Session(objects.add(Object::NetSession(session)))
                    }
                    Ok(None) => NetUseTarget::Null,
                    Err(failure) => NetUseTarget::Failure(failure.into()),
                },
            )
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaxploitRshellClient<'static>, caller| {
            rshell_client(
                world,
                &objects.metaxploit(message.metaxploit)?.access,
                &caller.terminal_computers,
                &message.address,
                message.port,
                &message.process_name,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: MetaxploitRshellServer, caller| {
            let access = &objects.metaxploit(message.metaxploit)?.access;
            let terminal_computers = &caller.terminal_computers;

            require_connection(world, terminal_computers).await?;

            let origin = world.public_ip(access.computer).await?;

            Ok(match world.rshell_clients(access.computer).await? {
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
                                computer: objects.add(Object::Computer(access.clone())),
                                shell: objects.add(Object::ReverseShell(access, client.pid)),
                            }
                        })
                        .collect(),
                ),
            })
        },
    );
    connection.operate(
        channel,
        async |state, connection, message: MetaxploitSniffer<'static>| {
            let start = state
                .cycle(async |world_state| {
                    state.session(connection.id, message.program)?;

                    let (world, objects) = world_state.of(connection.id);
                    let access = objects.metaxploit(message.metaxploit)?.clone().access;
                    let sniffer = world.start_sniffer(access.computer).await?;

                    Ok::<_, OperationError>(sniffer.map(|sniffer| {
                        (
                            access,
                            state.defer_job(connection.id, message.program, sniffer),
                        )
                    }))
                })
                .await?;
            let Some((access, end)) = start else {
                return Ok(None);
            };
            let WorkResult::Capture(packet) = end
                .await
                .map_err(|_| OperationError::Script(ScriptFailure::ProgramEnded))?
            else {
                return Err(OperationError::Script(ScriptFailure::ProgramEnded));
            };

            state
                .cycle(async |world_state| {
                    let session = state.session(connection.id, message.program)?;
                    let (world, objects) = world_state.of(connection.id);
                    let user = objects.session(session)?.user.clone();

                    capture(
                        world,
                        &access,
                        packet,
                        message.encode_source,
                        &message.program_path,
                        &user,
                    )
                    .await
                })
                .await
                .map(Some)
        },
    );
}
