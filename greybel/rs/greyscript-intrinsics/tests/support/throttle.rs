use std::time::Duration;

use duplex_core::{Channel, ChannelOptions};
use duplex_driver_memory::MemoryDriver;
use greyhack_world_operations::protocol::{
    EndProgram, GameServer, ObjectId, ProgramId, ProgramSession, ProgramThrottle, StartProgram,
};
use greyscript_intrinsics::ServerConnection;
use scheduler::{ManualClock, TokioScheduler};

const PROGRAM: ProgramId = ProgramId { id: 7 };

/// Lets the server's and the runtime's tasks run until they wait.
pub async fn settle() {
    for _ in 0..256 {
        tokio::task::yield_now().await;
    }
}

/// Passes `milliseconds` on `clock` in steps of 50 ms, letting the tasks run after each.
pub async fn pass(clock: &ManualClock, milliseconds: u64) {
    for _ in 0..milliseconds / 50 {
        clock.advance(Duration::from_millis(50));
        settle().await;
    }
}

/// A game server that starts every program as [`PROGRAM`], throttles it for `milliseconds` right after, and accepts its
/// end.
pub fn throttling_server(milliseconds: u32) -> ServerConnection {
    let (server, runtime) = MemoryDriver::pair(TokioScheduler::new());
    let server = Channel::<GameServer, _>::new(&server, ChannelOptions::default());
    let notifier = server.clone();

    server.handle(move |_: StartProgram, responder, _next| {
        responder.respond(ProgramSession {
            program: PROGRAM,
            shell: ObjectId { id: 0 },
            user: "player".into(),
            home: "/home/player".into(),
            path: "/home/player/script".into(),
        });

        let _ = notifier.send(&ProgramThrottle {
            program: PROGRAM,
            milliseconds,
        });
    });
    server.handle(|_: EndProgram, responder, _next| responder.respond(true));

    let connection = ServerConnection::new(Channel::new(&runtime, ChannelOptions::default()));

    std::mem::forget(server);

    connection
}
