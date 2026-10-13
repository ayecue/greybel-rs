//! Headless Grey Hack test utilities using the message-hook player and `greybel-runtime` to verify deployed builds
//! and imports.

use std::collections::BTreeMap;
use std::path::Path;
use std::pin::{Pin, pin};
use std::task::Poll;

use duplex_core::{Channel, ChannelOptions};
use duplex_test_support::future::poll;
use duplex_test_support::local::run_local;
use greybel_documents_host_provider::protocol::Editor as DocumentsEditor;
use greybel_io_web::protocol::Page;
use greybel_runtime::Runtime;
use greybel_test_support::file_editor::FileEditor;
use greybel_test_support::game::{Game, RUN_TIME};
use greybel_test_support::terminal::TerminalRecorder;

use super::MAIN;
use super::requests::EXTENSIONS;

/// Client runtime protocol bindings used by editor-side game checks.
mod runtime {
    duplex_core::include_protocol!("$PROTOCOLS/runtime", major = 1, role = client);
}

/// Starts headless Grey Hack and a player-capable plugin in `folder`, then invokes `test` with the game port.
pub fn in_game<F: Future<Output = ()>>(folder: impl AsRef<Path>, test: impl FnOnce(u16) -> F) {
    run_local(async {
        let (game, client) = Game::start_with_player(folder).await;

        client.shutdown().await;
        test(game.port()).await;
    });
}

/// Runs `code` through `greybel-runtime` on the game at `port` and returns `(completed, printed_lines)`.
pub async fn run_in_game(port: u16, code: &str) -> (bool, Vec<String>) {
    let (editor_transport, runtime_transport, queue) = duplex_test_support::Memory::transports();
    let mut serve: Pin<Box<dyn Future<Output = ()>>> =
        Box::pin(Runtime::new(&runtime_transport).serve());
    let editor: Channel<runtime::Editor, _> =
        Channel::new(&editor_transport, ChannelOptions::default());
    let page: Channel<Page, _> = Channel::new(&editor_transport, ChannelOptions::default());
    let documents: Channel<DocumentsEditor, _> =
        Channel::new(&editor_transport, ChannelOptions::default());
    let terminal = TerminalRecorder::serve(&page);

    FileEditor::serve(&documents, &[(MAIN, code)]);

    let launch = editor
        .send(&runtime::Launch {
            entry: MAIN.into(),
            root: None,
            extensions: EXTENSIONS.map(str::to_owned).to_vec().into(),
            environment: runtime::Environment::Game { port },
            program_name: "builder".into(),
            debug: false,
            stop_on_entry: false,
            params: Vec::new().into(),
            variables: BTreeMap::new(),
            strict_mode: false,
        })
        .expect("the launch is sent");
    let mut launch = pin!(launch);
    let deadline = std::time::Instant::now() + RUN_TIME;

    while std::time::Instant::now() < deadline {
        if let Poll::Ready(exit) = poll(&mut launch) {
            let printed = terminal.print_log.borrow().clone();

            return (exit == Ok(runtime::Exit::Completion), printed);
        }

        let _ = poll(&mut serve);

        queue.deliver();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    panic!("the check didn't end within {RUN_TIME:?}");
}
