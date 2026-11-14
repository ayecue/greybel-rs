//! Runs a program against a game server on the same thread on `wasm32-unknown-unknown`, the browser's runtime.

#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]

use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions};
use greybel_documents_memory_provider::MemoryProvider;
use greybel_interpreter::{InterpreterOptions, Stdout};
use greybel_symbols::names::UNNAMED_SOURCE;
use greyhack_world::{PlayerOptions, World, WorldOptions};
use greyhack_world_operations::WorldServer;
use greyscript_intrinsics::protocol::Program;
use greyscript_intrinsics::{GreyHackInterpreter, ServerConnection};
use scheduler::{ManualClock, WebScheduler};
use wasm_bindgen_test::wasm_bindgen_test;

#[path = "support/web.rs"]
mod support;
use support::{Lines, SCRIPT};

#[wasm_bindgen_test]
async fn should_launch_the_games_commands_on_the_web() {
    // Arrange
    let world = World::generate(&WorldOptions {
        seed: 7,
        networks: 2,
        player: PlayerOptions::default(),
    })
    .await
    .expect("a generated world");
    let server = WorldServer::new(world, Rc::new(ManualClock::constant()))
        .await
        .unwrap();
    let connection = ServerConnection::new(Channel::<Program, _>::new(
        &server.connect(WebScheduler),
        ChannelOptions::default(),
    ));
    let lines = Rc::new(Lines::default());
    let options = InterpreterOptions {
        stdout: Rc::clone(&lines) as Rc<dyn Stdout>,
        ..InterpreterOptions::new(Rc::new(ManualClock::constant()))
    };
    let provider: MemoryProvider = [(UNNAMED_SOURCE, SCRIPT)].into_iter().collect();
    let user = PlayerOptions::default().user;

    // Act
    let mut host = GreyHackInterpreter::start(&connection, &user, options, provider)
        .await
        .expect("the program starts");
    let result = host.run().await;

    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(
        lines.0.borrow().join("\n"),
        "/home/test\n1\ntest   \nguest\n<b>Usage: ls [opt:-l, -a, -la] [opt: path]</b>\nIt can be called without parameters."
    );
}
