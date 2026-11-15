//! Shared helpers for `greyhack-world-operations` integration tests.

use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, NextNotification};
use duplex_driver_memory::MemoryDriver;
use greyhack_behaviour::hash_password;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::Process;
use greyhack_world_operations::WorldServer;
use scheduler::{Clock, ManualClock, TokioScheduler};

pub mod admin_computer;
pub mod clock;
pub mod server;

/// Client-side world protocol used by test runtimes.
pub mod program {
    duplex_core::include_protocol!("$PROTOCOLS/world", major = 1, role = client);
}

use greybel_test_support::future::block_on;
use program::{
    Program, ProgramEnd, ProgramPrint, ProgramPrintRef, ProgramRouter, ProgramSession,
    ProgramThrottle, StartProgram,
};

/// Runtime channel and what the game server told it, in arrival order.
pub struct Runtime {
    pub channel: Channel<Program, MemoryDriver<TokioScheduler>>,
    pub notices: Rc<RefCell<Vec<Notice>>>,
}

/// A notification the game server sent a test runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    End(ProgramEnd),
    Throttle(ProgramThrottle),
    Print(ProgramPrint),
}

struct Notices(Rc<RefCell<Vec<Notice>>>);

impl ProgramRouter for Notices {
    fn on_program_end(&self, message: ProgramEnd, _next: NextNotification<ProgramEnd>) {
        self.0.borrow_mut().push(Notice::End(message));
    }

    fn on_program_print(
        &self,
        message: ProgramPrintRef<'_>,
        _next: NextNotification<ProgramPrint>,
    ) {
        self.0
            .borrow_mut()
            .push(Notice::Print(message.into_owned()));
    }

    fn on_program_throttle(
        &self,
        message: ProgramThrottle,
        _next: NextNotification<ProgramThrottle>,
    ) {
        self.0.borrow_mut().push(Notice::Throttle(message));
    }
}

/// Creates a server for a new player world with a constant clock.
#[must_use]
pub fn server() -> WorldServer {
    server_on(Rc::new(ManualClock::constant()))
}

/// Creates a server for a new player world using `clock`.
#[must_use]
pub fn server_on(clock: Rc<dyn Clock>) -> WorldServer {
    let world = block_on(World::new(&PlayerOptions::default())).expect("the world generates");

    block_on(WorldServer::new(world, clock)).expect("the server restores payments")
}

/// Creates a server for a new player world whose coin `Ether` was created by `test` with the password `test`, with
/// its next mining round an hour of world time away.
#[must_use]
pub fn server_with_ether() -> WorldServer {
    let world = block_on(World::new(&PlayerOptions::default())).expect("the world generates");

    block_on(async {
        let now = world.handle.time_seconds();
        let mut coin = world
            .modules
            .economy
            .coins
            .coin_by_name("Ether")
            .await
            .unwrap()
            .expect("Ether");

        "test".clone_into(&mut coin.creator_user);
        coin.creator_password_hash = hash_password("test");
        coin.cycle_hours = 1.0;
        coin.last_cycle_at = Some(now);
        world.modules.economy.coins.save_coin(&coin).await.unwrap();
    });

    block_on(WorldServer::new(world, Rc::new(ManualClock::constant())))
        .expect("the server restores payments")
}

/// Connects a same-thread test runtime to `server`.
#[must_use]
pub fn connect(server: &WorldServer) -> Runtime {
    let channel = Channel::new(&server.connect_on_tokio(), ChannelOptions::default());
    let notices = Rc::new(RefCell::new(Vec::new()));

    Notices(Rc::clone(&notices)).attach(&channel);

    Runtime { channel, notices }
}

/// Returns the default player username.
#[must_use]
pub fn player() -> String {
    PlayerOptions::default().user
}

impl Runtime {
    /// Starts `path` as the default player user and returns its session.
    pub async fn start(&self, path: &str) -> ProgramSession {
        self.channel
            .send(&StartProgram {
                user: player().into(),
                path: Some(path.to_owned().into()),
                safe_terminal: false,
            })
            .expect("the program starts")
            .await
            .expect("the game server responds")
    }

    /// The program ends the game server sent, in order.
    pub fn ended(&self) -> Vec<ProgramEnd> {
        self.notices
            .borrow()
            .iter()
            .filter_map(|notice| match notice {
                Notice::End(end) => Some(*end),
                Notice::Throttle(_) | Notice::Print(_) => None,
            })
            .collect()
    }
}

/// Returns process `name` on the player's computer, if present.
#[must_use]
pub fn process(server: &WorldServer, name: &str) -> Option<Process> {
    block_on(async {
        let world = server.world().await;
        let player = world.handle.player();

        world
            .modules
            .core
            .computers
            .processes
            .processes(player)
            .await
            .expect("the world reads")
            .into_iter()
            .find(|process| process.name == name)
    })
}

/// Ends the process `name` of the player's computer, as `close_program` does.
pub async fn end_process(server: &WorldServer, name: &str) {
    let pid = process(server, name).expect("the process").pid;

    server
        .change_world(async |world| {
            let player = world.handle.player();

            world
                .modules
                .core
                .computers
                .processes
                .end_process(player, pid)
                .await
                .expect("the world writes");
        })
        .await;
}

/// Lets the server's and the runtimes' tasks run until they wait.
pub async fn settle() {
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }
}
