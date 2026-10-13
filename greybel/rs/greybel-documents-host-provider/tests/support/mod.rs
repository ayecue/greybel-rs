//! Shared helpers for `greybel-documents-host-provider` integration tests.

#![allow(dead_code)]

use std::rc::Rc;

use duplex_core::Channel;
use duplex_test_support::{Memory, MemoryQueue};
use greybel_documents::{PathResolution, Uri};
use greybel_documents_host_provider::HostProvider;
use greybel_documents_host_provider::protocol::{Editor, ReadFile};
use greybel_test_support::file_editor::FileEditor;

/// Host provider connected to an editor serving `files`, retaining the editor channel for the harness lifetime.
pub struct Connected {
    pub provider: HostProvider<Memory>,
    pub files: Rc<FileEditor>,
    queue: MemoryQueue,
    editor: Channel<Editor, Memory>,
}

impl Connected {
    /// Drives `future` while delivering queued memory batches until it is ready.
    pub fn drive<F: Future>(&self, future: F) -> F::Output {
        self.queue.drive(future)
    }
}

pub fn connected(files: &[(&str, &str)]) -> Connected {
    connected_resolving(files, &["src"])
}

/// Creates a connected provider using `extensions` for import resolution and `file:///work` as its root.
pub fn connected_resolving(files: &[(&str, &str)], extensions: &[&str]) -> Connected {
    let (editor, provider, queue) = Memory::connected_pair::<Editor>();
    let files = FileEditor::serve(&editor, files);
    let provider = HostProvider::new(provider, PathResolution::new(extensions.iter().copied()))
        .with_folders([uri("file:///work")]);

    Connected {
        provider,
        files,
        queue,
        editor,
    }
}

/// Creates a connected provider serving [`FILES`] with both `src` and `ms` extensions.
pub fn connected_with_extensions() -> Connected {
    connected_resolving(FILES, &["src", "ms"])
}

/// Creates a provider whose editor drops every `ReadFile` responder to simulate request failure.
pub fn connected_to_a_failing_editor()
-> (HostProvider<Memory>, MemoryQueue, Channel<Editor, Memory>) {
    let (editor, provider, queue) = Memory::connected_pair::<Editor>();

    editor.handle(|_: ReadFile, response, _next| drop(response));

    (
        HostProvider::new(provider, PathResolution::new(["src"])),
        queue,
        editor,
    )
}

pub fn uri(value: &str) -> Uri {
    Uri::parse(value).expect("a URI")
}

pub const FILES: &[(&str, &str)] = &[
    (
        "file:///work/main.src",
        "#import lib from \"lib\"\nprint lib",
    ),
    ("file:///work/lib.src", "lib = 1"),
    ("file:///work/My%20Folder/a.src", "a = 1"),
];
