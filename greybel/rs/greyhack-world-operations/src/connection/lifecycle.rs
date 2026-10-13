//! Program lifecycle handlers. Each runtime program runs as a computer process until the runtime ends it.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::path;

use super::Connection;
use crate::connection::objects::{Access, Object};
use crate::protocol::{EndProgram, GameServer, ProgramSession, StartProgram};

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
                .cycle(async |state_of_world| {
                    let (world, objects) = state_of_world.of(connection.id);
                    let computer = world.player();
                    let pid = if world.computer(computer).await?.is_some() {
                        world
                            .spawn_process(computer, &message.user, &name, None)
                            .await?
                    } else {
                        0
                    };
                    // `MainClass.Main`: the previous address of a program on the player's computer is its
                    // public address.
                    let prev_ip = world.public_ip(computer).await?;
                    let shell = objects.add(Object::Shell(Access {
                        computer,
                        user: message.user.to_string(),
                        ftp: false,
                        prev_ip,
                        bounces: 0,
                    }));

                    Ok(ProgramSession {
                        program: state.add_program(
                            connection.id,
                            Some((computer, pid)),
                            shell,
                            None,
                        )?,
                        shell,
                        user: message.user.to_string(),
                        home,
                        path: program_path,
                    })
                })
                .await
        },
    );
    connection.operate(channel, async |state, connection, message: EndProgram| {
        state
            .cycle(async |world_state| {
                state
                    .end_program(world_state, connection.id, message.program, message.reason)
                    .await;
            })
            .await;

        Ok(!state.has_programs(connection.id) && state.save().await)
    });
}
