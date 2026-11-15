//! Program lifecycle handlers. Each runtime program runs as a computer process until the runtime ends it.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::path;
use greyhack_world_core::{Access, AccessHandle, AccessKind};
use greyhack_world_programs::protocol::{
    EndProgram, GameServer, ObjectId, ProgramSession, ScriptFailure, StartProgram,
};

use super::Connection;
use crate::OperationError;

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.operate(
        channel,
        async |state, connection, message: StartProgram<'static>| {
            let home = path::home(&message.user);
            let program_path = message
                .path
                .as_deref()
                .map_or_else(|| path::join(&home, "script"), str::to_owned);
            let name = path::file_name(&program_path).to_owned();

            state
                .cycle(
                    async |world_state| {
                        let world = &mut world_state.world;

                        // After a game over (`Database.GameOver`) the player has no computer to run a program on:
                        // the client has disconnected.
                        if !world.has_player() {
                            return Err(OperationError::Script(ScriptFailure::ProgramEnded));
                        }

                        let computer = world.handle.player();
                        // `GreyScriptHelperServer.PrepareTerminalServerRpc`: the program is in the safe-mode terminal
                        // only when it is typed into it, and the terminal runs.
                        let safe_terminal = message.safe_terminal
                            && world
                                .modules
                                .core
                                .computers
                                .processes
                                .in_safe_mode(computer)
                                .await?;
                        let pid = if world
                            .modules
                            .core
                            .networks
                            .topology
                            .computer(computer)
                            .await?
                            .is_some()
                        {
                            world
                                .modules
                                .core
                                .computers
                                .processes
                                .spawn_process(computer, &message.user, &name, None)
                                .await?
                        } else {
                            0
                        };
                        // `MainClass.Main`: the previous address of a program on the player's computer is its
                        // public address.
                        let prev_ip = world
                            .modules
                            .core
                            .networks
                            .topology
                            .public_ip(computer)
                            .await?;
                        let program = state.add_root(
                            world_state,
                            connection.id,
                            Some((computer, pid)),
                            ObjectId { id: 0 },
                            safe_terminal,
                        )?;
                        let shell = world_state
                            .trees
                            .objects_of_connection(connection.id)
                            .insert(AccessHandle {
                                access: Access {
                                    computer,
                                    user: message.user.to_string(),
                                    ftp: false,
                                    prev_ip,
                                    bounces: 0,
                                },
                                kind: AccessKind::Shell,
                            });

                        let tree = world_state
                            .trees
                            .of_program(program)
                            .expect("the new root is indexed");

                        world_state
                            .trees
                            .get_mut(tree)
                            .expect("the new root tree exists")
                            .set_session(program, shell);

                        Ok(ProgramSession {
                            program,
                            shell,
                            user: message.user.to_string(),
                            home,
                            path: program_path,
                        })
                    },
                    true,
                )
                .await
        },
    );
    connection.operate(channel, async |state, connection, message: EndProgram| {
        state
            .cycle(
                async |world_state| {
                    state
                        .end_program(world_state, connection.id, message.program, message.reason)
                        .await;
                },
                true,
            )
            .await;

        Ok(!state.has_programs(connection.id).await && state.save().await)
    });
}
