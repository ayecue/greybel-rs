//! Operations on shell objects (`ShellIntrinsics`).

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greybel_source_map::{Mapping, SourceMap};
use greyhack_behaviour::{BASH_PATH, GUEST_USER, path};
use greyhack_world::{Launch, LaunchedProgram, TerminalFailure, TerminalProgram, World};

use super::require_connection;
use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{Access, Object};
use crate::protocol::{
    BuildSource, Failure, GameServer, LaunchFailure, LaunchOutcome, Outcome, ProgramSession,
    ServiceConnection, ShellBuild, ShellCompleteEncryptedLogin, ShellConnectService, ShellFailure,
    ShellLaunch, ShellPing, ShellScp, ShellStartTerminal, ShellWriteBinary,
};
use crate::server::TerminalInfo;

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.change(
        channel,
        async |world, objects, message: ShellPing<'static>, _| {
            Ok(world
                .ping(objects.shell(message.shell)?.computer, &message.address)
                .await?)
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ShellScp<'static>, _| {
            let to = match message.to {
                Some(to) => Some(objects.shell(to)?),
                None => None,
            };

            scp(
                world,
                objects.shell(message.from)?,
                to,
                &message.source,
                &message.destination,
                message.upload,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ShellConnectService<'static>, caller| {
            let terminal_computers = &caller.terminal_computers;
            let login = ServiceLogin {
                address: message.address.as_deref(),
                port: message.port,
                user: message.user.as_deref(),
                password: message.password.as_deref(),
                service: message.service.as_deref(),
            };

            let service = connect_service(
                world,
                objects.shell(message.shell)?,
                terminal_computers,
                &login,
            )
            .await?;

            Ok(match service {
                ServiceAccess::Failure(failure) => ServiceConnection::Failure(failure),
                ServiceAccess::Shell(access) => ServiceConnection::Shell {
                    ftp: access.ftp,
                    shell: objects.add(Object::Shell(access)),
                },
                ServiceAccess::Encryption {
                    access,
                    message,
                    failure,
                    encode,
                    decode,
                } => ServiceConnection::Encrypted {
                    user: access.user.clone(),
                    guest_shell: objects.add(Object::Shell(Access {
                        computer: access.computer,
                        user: GUEST_USER.to_owned(),
                        ftp: false,
                        prev_ip: access.prev_ip.clone(),
                        bounces: access.bounces,
                    })),
                    message,
                    failure,
                    encode,
                    decode,
                },
            })
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ShellCompleteEncryptedLogin<'static>, caller| {
            let terminal_computers = &caller.terminal_computers;
            let source = objects.shell(message.shell)?;
            let target = objects.shell(message.target)?;
            let result =
                complete_encrypted_login(world, source, target, terminal_computers, &message)
                    .await?;

            Ok(match result {
                Ok(access) => crate::protocol::ServiceLoginResult::Shell {
                    ftp: access.ftp,
                    shell: objects.add(Object::Shell(access)),
                },
                Err(failure) => crate::protocol::ServiceLoginResult::Failure(failure),
            })
        },
    );
    connection.read(
        channel,
        async |world, objects, message: ShellBuild<'static>, caller| {
            build(
                world,
                objects.shell(message.shell)?,
                objects.session(caller.session)?,
                &message.source,
                &message.folder,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: ShellWriteBinary<'static>, _| {
            write_binary(
                world,
                objects.shell(message.shell)?,
                &message.folder,
                &message.name,
                &message.source,
                message.allow_import,
                source_map(&message.source_map),
            )
            .await
        },
    );

    connection.operate(
        channel,
        async |state, connection, message: ShellLaunch<'static>| {
            state
                .cycle_for(connection.id, message.program, async |world_state| {
                    let session_id = state.session(connection.id, message.program)?;

                    let (world, objects) = world_state.of(connection.id);
                    let session_computer = objects.session(session_id)?.computer;
                    let shell = objects.shell(message.shell)?.clone();
                    let launch = world
                        .launch_program(
                            shell.computer,
                            &shell.user,
                            &message.path,
                            &message.params,
                            state.depth(message.program),
                        )
                        .await?;

                    Ok(match launch {
                        Launch::Gone => LaunchOutcome::Refusal,
                        Launch::Refusal(failure) => {
                            LaunchOutcome::Failure(Failure::Launch(failure.into()))
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
                            let session = objects.add(Object::Shell(shell));
                            let program = state.add_program(
                                connection.id,
                                Some(process),
                                session,
                                Some(message.program),
                            )?;

                            // `ShellIntrinsics.launch`: the program's shell on a company's computer starts an
                            // active trace, which ends with the program.
                            world
                                .trace_shell(shell_at.0, shell_at.1, u64::from(program.id))
                                .await?;

                            LaunchOutcome::Program {
                                session: ProgramSession {
                                    program,
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
                })
                .await
        },
    );

    connection.operate(
        channel,
        async |state, connection, message: ShellStartTerminal| {
            state
                .cycle_for(connection.id, message.program, async |world_state| {
                    let launcher = state.session(connection.id, message.program)?;
                    let (world, objects) = world_state.of(connection.id);
                    let launcher_bounces = objects.session(launcher)?.bounces;
                    let shell = objects.shell(message.shell)?.clone();

                    // Register the shell computer for the tree even if terminal startup fails
                    // (`HelperScript.AddPendingShell`).
                    state.add_terminal_computer(connection.id, message.program, shell.computer);

                    // `ShellIntrinsics.start_terminal` adds the shell's computer to the terminal process's
                    // main-terminal list before it checks anything (`Proceso.AddMainTerminalRemote`).
                    state.add_remote_terminal(connection.id, message.program, shell.computer);

                    Ok(
                        match world
                            .terminal_program(
                                shell.computer,
                                &shell.user,
                                state.depth(message.program),
                            )
                            .await?
                        {
                            Err(failure) => LaunchOutcome::Failure(match failure {
                                TerminalFailure::ComputerUnavailable => {
                                    Failure::Shell(ShellFailure::ComputerUnavailable)
                                }
                                TerminalFailure::TooManyStackCalls => {
                                    Failure::Launch(LaunchFailure::TooManyStackCalls)
                                }
                            }),
                            Ok(TerminalProgram { source, source_map }) => {
                                // A launched program becoming a terminal closes its original process
                                // This matches `ShellIntrinsics.start_terminal`.
                                if let Some((computer, pid)) =
                                    state.take_launched_process(connection.id, message.program)
                                {
                                    world.end_process(computer, pid).await?;
                                }

                                let (user, home) = (shell.user.clone(), path::home(&shell.user));
                                let info = TerminalInfo {
                                    host: shell.computer,
                                    bounces: shell.bounces,
                                    launcher_bounces,
                                };
                                let shell_at = (shell.computer, shell.bounces);
                                let session = objects.add(Object::Shell(shell));
                                let program = state.add_terminal_program(
                                    connection.id,
                                    session,
                                    message.program,
                                    info,
                                )?;

                                // `ShellIntrinsics.start_terminal`: the terminal's shell on a company's
                                // computer starts an active trace for the script that started it.
                                world
                                    .trace_shell(
                                        shell_at.0,
                                        shell_at.1,
                                        u64::from(message.program.id),
                                    )
                                    .await?;

                                LaunchOutcome::Program {
                                    session: ProgramSession {
                                        program,
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
                })
                .await
        },
    );
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
    world: &mut World,
    from: &Access,
    to: Option<&Access>,
    source: &str,
    destination: &str,
    upload: bool,
) -> Result<Outcome> {
    let result = world
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
            greyhack_world::FileFailure::PartialCopy.into(),
        )),
        Ok(false) => Outcome::True,
        Err(failure) => Outcome::Failure(Failure::File(failure.into())),
    })
}

/// Validates a build from `source` into `folder` as `shell`, expanding imports through `session`.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn build(
    world: &World,
    shell: &Access,
    session: &Access,
    source: &str,
    folder: &str,
) -> Result<BuildSource> {
    Ok(
        match world
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
            Err(failure) => BuildSource::Failure(Failure::Shell(ShellFailure::try_from(failure)?)),
        },
    )
}

/// Writes compiled `source` to `folder`/`name`, returning any shell failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn write_binary(
    world: &mut World,
    shell: &Access,
    folder: &str,
    name: &str,
    source: &str,
    allow_import: bool,
    source_map: SourceMap,
) -> Result<Outcome> {
    Ok(
        match world
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
            Err(failure) => Outcome::Failure(Failure::Shell(ShellFailure::try_from(failure)?)),
        },
    )
}

/// Converts `map` to protocol form.
pub fn protocol_source_map(map: &SourceMap) -> crate::protocol::SourceMap {
    crate::protocol::SourceMap {
        sources: map.sources().to_vec(),
        mappings: map
            .mappings()
            .iter()
            .map(|mapping| crate::protocol::SourceMapping {
                first: mapping.first,
                last: mapping.last,
                source: mapping.source,
                line: mapping.line,
            })
            .collect(),
    }
}

/// Converts protocol `map` to its runtime source-map representation.
pub fn source_map(map: &crate::protocol::SourceMap) -> SourceMap {
    SourceMap::new(
        map.sources.clone(),
        map.mappings
            .iter()
            .map(|mapping| Mapping::new(mapping.first, mapping.last, mapping.source, mapping.line))
            .collect(),
    )
}
