//! Operations on shell objects (`ShellIntrinsics`).

use greybel_source_map::{Mapping, SourceMap};
use greyhack_behaviour::{BASH_PATH, GUEST_USER, path};
use greyhack_world_core::{Access, AccessHandle, AccessKind, ObjectTableExt as _};
use greyhack_world_router::{CycleAccess, RouteTable, TerminalInfo};

use crate::routes::protocol::{
    BuildSource, ConnectionFailure, Failure, IntoProtocol as _, LaunchFailure, LaunchOutcome,
    LoginFailure, Outcome, PrintLine, ProgramSession, ServiceConnection, ServiceLoginResult,
    ShellBuild, ShellCompleteEncryptedLogin, ShellConnectService, ShellFailure, ShellLaunch,
    ShellPing, ShellScp, ShellStartTerminal, ShellWriteBinary,
};
use crate::routes::{OperationError, Result};
use crate::{Launch, LaunchedProgram, TerminalFailure, TerminalProgram};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellPing<'static>| {
            let world = &*ctx.modules;

            Ok(world
                .module::<greyhack_world_core::CoreModule>()
                .networks
                .reach
                .ping(
                    ctx.trees.objects(call.tree).shell(message.shell)?.computer,
                    &message.address,
                )
                .await?)
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellScp<'static>| {
            let world = &*ctx.modules;
            let to = match message.to {
                Some(to) => Some(&ctx.trees.objects(call.tree).shell(to)?),
                None => None,
            };

            scp(
                world,
                &ctx.trees.objects(call.tree).shell(message.from)?,
                to,
                &message.source,
                &message.destination,
                message.upload,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellConnectService<'static>| {
            let world = &*ctx.modules;
            let terminal_computers = &ctx.terminal_computers(call);
            let login = ServiceLogin {
                address: message.address.as_deref(),
                port: message.port,
                user: message.user.as_deref(),
                password: message.password.as_deref(),
                service: message.service.as_deref(),
            };

            let service = connect_service(
                world,
                &ctx.trees.objects(call.tree).shell(message.shell)?,
                terminal_computers,
                &login,
            )
            .await?;

            Ok(match service {
                ServiceAccess::Failure(Failure::Login(failure))
                    if login_reaches_computer(&failure) =>
                {
                    ctx.print(call, PrintLine::LoginFailure(failure));

                    ServiceConnection::Null
                }
                ServiceAccess::Failure(failure) => ServiceConnection::Failure(failure),
                ServiceAccess::Shell(access) => ServiceConnection::Shell {
                    ftp: access.ftp,
                    shell: ctx.trees.objects(call.tree).insert(AccessHandle {
                        access,
                        kind: AccessKind::Shell,
                    }),
                },
                ServiceAccess::Encryption {
                    access,
                    message,
                    failure,
                    encode,
                    decode,
                } => {
                    if message {
                        ctx.print(call, PrintLine::EncryptionEnabled);
                    }

                    ServiceConnection::Encrypted {
                        user: access.user.clone(),
                        guest_shell: ctx.trees.objects(call.tree).insert(AccessHandle {
                            access: Access {
                                computer: access.computer,
                                user: GUEST_USER.to_owned(),
                                ftp: false,
                                prev_ip: access.prev_ip.clone(),
                                bounces: access.bounces,
                            },
                            kind: AccessKind::Shell,
                        }),
                        failure,
                        encode,
                        decode,
                    }
                }
            })
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellCompleteEncryptedLogin<'static>| {
            let world = &*ctx.modules;
            let terminal_computers = &ctx.terminal_computers(call);
            let source = &ctx.trees.objects(call.tree).shell(message.shell)?;
            let target = &ctx.trees.objects(call.tree).shell(message.target)?;
            let result =
                complete_encrypted_login(world, source, target, terminal_computers, &message)
                    .await?;

            Ok(match result {
                Ok(access) => ServiceLoginResult::Shell {
                    ftp: access.ftp,
                    shell: ctx.trees.objects(call.tree).insert(AccessHandle {
                        access,
                        kind: AccessKind::Shell,
                    }),
                },
                Err(failure) => {
                    ctx.print(call, PrintLine::LoginFailure(failure));

                    ServiceLoginResult::Null
                }
            })
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellBuild<'static>| {
            let world = &*ctx.modules;

            build(
                world,
                &ctx.trees.objects(call.tree).shell(message.shell)?,
                &ctx.trees.objects(call.tree).session(
                    ctx.session(call)
                        .expect("dispatch checked the active program"),
                )?,
                &message.source,
                &message.folder,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellWriteBinary<'static>| {
            let world = &*ctx.modules;

            write_binary(
                world,
                &ctx.trees.objects(call.tree).shell(message.shell)?,
                &message.folder,
                &message.name,
                &message.source,
                message.allow_import,
                source_map(&message.source_map),
            )
            .await
        },
    );

    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellLaunch<'static>| {
            let cycles = ctx.cycles();
            let program = call.program;

            CycleAccess::change_dynamic(
                cycles,
                program,
                true,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let world = &*ctx.modules;
                    let session_id = ctx
                        .trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .session(program)
                        .expect("dispatch checked the active program");
                    let session_computer =
                        ctx.trees.objects(call.tree).session(session_id)?.computer;
                    let shell = ctx.trees.objects(call.tree).shell(message.shell)?.clone();
                    let launch = world
                        .module::<crate::ShellModule>()
                        .launches
                        .launch_program(
                            shell.computer,
                            &shell.user,
                            &message.path,
                            &message.params,
                            ctx.depth(call, program),
                            ctx.safe_terminal(call),
                        )
                        .await?;

                    Ok(match launch {
                        Launch::Missing => LaunchOutcome::Refusal,
                        Launch::Refusal(failure) => {
                            ctx.print(
                                call,
                                PrintLine::LaunchFailure(Failure::Launch(failure.into_protocol())),
                            );

                            LaunchOutcome::Refusal
                        }
                        Launch::Program(LaunchedProgram {
                            process,
                            path,
                            source,
                            source_map: lines,
                        }) => {
                            let user = shell.user.clone();
                            let home = if shell.computer == session_computer {
                                message.current.to_string()
                            } else {
                                path::home(&shell.user)
                            };
                            let shell_at = (shell.computer, shell.bounces);
                            let session = ctx.trees.objects(call.tree).insert(AccessHandle {
                                access: shell,
                                kind: AccessKind::Shell,
                            });
                            let child =
                                CycleAccess::push_dynamic::<W, greyhack_world_core::WorldError>(
                                    cycles,
                                    call.tree,
                                    Some(process),
                                    session,
                                    program,
                                    greyhack_world_router::ProgramKind::Script,
                                )?;

                            // `ShellIntrinsics.launch`: the program's shell on a company's computer starts an
                            // active trace, which ends with the program.
                            world
                                .module::<crate::ShellModule>()
                                .connections
                                .trace_shell(shell_at.0, shell_at.1, u64::from(child.id))
                                .await?;

                            LaunchOutcome::Program {
                                session: ProgramSession {
                                    program: child,
                                    user,
                                    home,
                                    shell: session,
                                    path,
                                },
                                source,
                                source_map: protocol_source_map(&lines),
                            }
                        }
                    })
                },
            )
            .await
        },
    );

    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: ShellStartTerminal| {
            let cycles = ctx.cycles();
            let program = call.program;

            CycleAccess::change_dynamic(
                cycles,
                program,
                true,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let world = &*ctx.modules;
                    let launcher = ctx
                        .trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .session(program)
                        .expect("dispatch checked the active program");
                    let launcher_bounces = ctx.trees.objects(call.tree).session(launcher)?.bounces;
                    let shell = ctx.trees.objects(call.tree).shell(message.shell)?.clone();

                    // Register the shell computer for the tree even if terminal startup fails
                    // (`HelperScript.AddPendingShell`).
                    ctx.trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .terminals
                        .push(shell.computer);

                    // `ShellIntrinsics.start_terminal` adds the shell's computer to the terminal process's
                    // main-terminal list before it checks anything (`Proceso.AddMainTerminalRemote`).
                    ctx.trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .add_remote_terminal(shell.computer);

                    Ok(
                        match world
                            .module::<crate::ShellModule>()
                            .launches
                            .terminal_program(shell.computer, &shell.user, ctx.depth(call, program))
                            .await?
                        {
                            Err(TerminalFailure::ComputerUnavailable) => LaunchOutcome::Failure(
                                Failure::Shell(ShellFailure::ComputerUnavailable),
                            ),
                            Err(TerminalFailure::TooManyStackCalls) => {
                                ctx.print(
                                    call,
                                    PrintLine::LaunchFailure(Failure::Launch(
                                        LaunchFailure::TooManyStackCalls,
                                    )),
                                );

                                LaunchOutcome::Refusal
                            }
                            Ok(TerminalProgram { source, source_map }) => {
                                // A launched program becoming a terminal closes its original process
                                // This matches `ShellIntrinsics.start_terminal`.
                                let launched_process = ctx
                                    .trees
                                    .get_mut(call.tree)
                                    .expect("the indexed tree exists")
                                    .take_launched_process(program);

                                if let Some((computer, pid)) = launched_process {
                                    world
                                        .module::<greyhack_world_core::CoreModule>()
                                        .computers
                                        .processes
                                        .end_process(computer, pid)
                                        .await?;
                                }

                                let (user, home) = (shell.user.clone(), path::home(&shell.user));
                                let info = TerminalInfo {
                                    host: shell.computer,
                                    bounces: shell.bounces,
                                    launcher_bounces,
                                };
                                let shell_at = (shell.computer, shell.bounces);
                                let session = ctx.trees.objects(call.tree).insert(AccessHandle {
                                    access: shell,
                                    kind: AccessKind::Shell,
                                });
                                let terminal = CycleAccess::push_dynamic::<
                                    W,
                                    greyhack_world_core::WorldError,
                                >(
                                    cycles,
                                    call.tree,
                                    None,
                                    session,
                                    program,
                                    greyhack_world_router::ProgramKind::Terminal(info),
                                )?;

                                // `ShellIntrinsics.start_terminal`: the terminal's shell on a company's
                                // computer starts an active trace for the script that started it.
                                world
                                    .module::<crate::ShellModule>()
                                    .connections
                                    .trace_shell(shell_at.0, shell_at.1, u64::from(program.id))
                                    .await?;

                                LaunchOutcome::Program {
                                    session: ProgramSession {
                                        program: terminal,
                                        user,
                                        home,
                                        shell: session,
                                        path: BASH_PATH.to_owned(),
                                    },
                                    source,
                                    source_map: protocol_source_map(&source_map),
                                }
                            }
                        },
                    )
                },
            )
            .await
        },
    );
}

/// Whether `connect_service` prints `failure` after reaching the target computer (`PlayerUtils.ConnectService`).
fn login_reaches_computer(failure: &LoginFailure) -> bool {
    matches!(
        failure,
        LoginFailure::Connection(ConnectionFailure::Lan(_) | ConnectionFailure::Startup(_))
            | LoginFailure::IncorrectPassword
            | LoginFailure::AccountInactive
    )
}

mod connect;

pub use connect::{ServiceAccess, ServiceLogin, complete_encrypted_login, connect_service};

/// Copies `source` between `from` and `to`. A missing `to` uses the player's first-user computer;
/// `upload` reverses the copy direction.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn scp(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    from: &Access,
    to: Option<&Access>,
    source: &str,
    destination: &str,
    upload: bool,
) -> Result<Outcome> {
    let result = world
        .module::<crate::ShellModule>()
        .builds
        .scp(
            (from.computer, &from.user),
            to.map(|to| (to.computer, to.user.as_str())),
            source,
            destination,
            upload,
        )
        .await?;

    Ok(match result {
        Ok(true) => Outcome::Failure(Failure::File(
            crate::FileFailure::PartialCopy.into_protocol(),
        )),
        Ok(false) => Outcome::True,
        Err(failure) => Outcome::Failure(Failure::File(failure.into_protocol())),
    })
}

/// Validates a build from `source` into `folder` as `shell`, expanding imports through `session`.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn build(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    shell: &Access,
    session: &Access,
    source: &str,
    folder: &str,
) -> Result<BuildSource> {
    Ok(
        match world
            .module::<crate::ShellModule>()
            .builds
            .build_source(
                (shell.computer, &shell.user),
                (session.computer, &session.user),
                source,
                folder,
            )
            .await?
        {
            Ok(build_result) => BuildSource::Code {
                source: build_result.source,
                name: build_result.name,
                source_map: protocol_source_map(&build_result.source_map),
            },
            // Return `PlayerUtils.ImportCode` failures as compiler-error descriptions from `build`.
            Err(failure) => BuildSource::Failure(Failure::Shell(
                <ShellFailure as crate::routes::protocol::TryProtocolFrom<_>>::try_from(failure)
                    .map_err(OperationError::Script)?,
            )),
        },
    )
}

/// Writes compiled `source` to `folder`/`name`, returning any shell failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn write_binary(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    shell: &Access,
    folder: &str,
    name: &str,
    source: &str,
    allow_import: bool,
    source_map: SourceMap,
) -> Result<Outcome> {
    Ok(
        match world
            .module::<crate::ShellModule>()
            .builds
            .write_binary(
                shell.computer,
                &shell.user,
                folder,
                name,
                (source, allow_import, source_map),
            )
            .await?
        {
            Ok(()) => Outcome::True,
            Err(failure) => Outcome::Failure(Failure::Shell(
                <ShellFailure as crate::routes::protocol::TryProtocolFrom<_>>::try_from(failure)
                    .map_err(OperationError::Script)?,
            )),
        },
    )
}

/// Converts `map` to protocol form.
pub fn protocol_source_map(map: &SourceMap) -> crate::routes::protocol::SourceMap {
    crate::routes::protocol::SourceMap {
        sources: map.sources().to_vec(),
        mappings: map
            .mappings()
            .iter()
            .map(|mapping| crate::routes::protocol::SourceMapping {
                first: mapping.first,
                last: mapping.last,
                source: mapping.source,
                line: mapping.line,
            })
            .collect(),
    }
}

/// Converts protocol `map` to its runtime source-map representation.
pub fn source_map(map: &crate::routes::protocol::SourceMap) -> SourceMap {
    SourceMap::new(
        map.sources.clone(),
        map.mappings
            .iter()
            .map(|mapping| Mapping::new(mapping.first, mapping.last, mapping.source, mapping.line))
            .collect(),
    )
}
