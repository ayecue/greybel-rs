//! Shared helpers for `greybel-builder` integration tests.

#![allow(dead_code)]

pub mod editor;
#[cfg(feature = "game-tests")]
pub mod game;
pub mod requests;

use std::future::Future;
use std::pin::{Pin, pin};
use std::task::Poll;

use duplex_core::{Channel, ChannelOptions, Exchange, Message};
use duplex_test_support::future::poll;
use duplex_test_support::{Memory, MemoryQueue};
use editor::Editor;
use greybel_builder::Builder;
use greybel_documents_host_provider::protocol::Editor as DocumentsEditor;
use greybel_test_support::file_editor::FileEditor;
use greybel_test_support::game::RUN_TIME;

/// Response future returned when the editor sends a message of type `M`.
type Pending<M> = <<M as Message>::Exchange as Exchange>::Handle;

/// Output type of the builder's response future for a message of type `M`.
type Response<M> = <Pending<M> as Future>::Output;

pub const MAIN: &str = "file:///project/main.src";
pub const LIBRARY: &str = "file:///project/lib.src";

/// In-memory editor and builder pair with an editor-hosted document service.
pub struct Harness {
    queue: MemoryQueue,
    pub editor: Channel<Editor, Memory>,
    serve: Pin<Box<dyn Future<Output = ()>>>,
    _documents: Channel<DocumentsEditor, Memory>,
}

impl Harness {
    pub fn new(files: &[(&str, &str)]) -> Self {
        let (editor_transport, builder_transport, queue) = Memory::transports();
        let serve = Box::pin(Builder::new(&builder_transport).serve());
        let editor = Channel::new(&editor_transport, ChannelOptions::default());
        let documents = Channel::new(&editor_transport, ChannelOptions::default());

        FileEditor::serve(&documents, files);

        Self {
            queue,
            editor,
            serve,
            _documents: documents,
        }
    }

    /// Drives `future` while delivering memory batches and polling the builder until it is ready.
    pub fn wait<F: Future>(&mut self, future: F) -> F::Output {
        let serve = &mut self.serve;

        self.queue.drive_with(future, || {
            let _ = poll(serve);
        })
    }

    /// Sends `message` and waits for the builder's response.
    pub fn request<M>(&mut self, message: &M) -> Response<M>
    where
        M: Message<Sender = Editor>,
        Pending<M>: Future,
    {
        let pending = self.editor.send(message).expect("sent");

        self.wait(pending)
    }

    /// Sends a request and polls the builder with 5 ms sleeps in a Tokio runtime until it responds or [`RUN_TIME`]
    /// elapses. Used for operations waiting on real I/O, such as Grey Hack.
    pub async fn request_on_io<M>(&mut self, message: &M) -> Response<M>
    where
        M: Message<Sender = Editor>,
        Pending<M>: Future,
    {
        let pending = self.editor.send(message).expect("sent");
        let mut pending = pin!(pending);
        let deadline = std::time::Instant::now() + RUN_TIME;

        while std::time::Instant::now() < deadline {
            if let Poll::Ready(output) = poll(&mut pending) {
                return output;
            }

            let _ = poll(&mut self.serve);

            self.queue.deliver();
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        panic!("nothing happened within {RUN_TIME:?}");
    }
}
