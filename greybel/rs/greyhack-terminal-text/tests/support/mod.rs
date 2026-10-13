//! Shared helpers for `greyhack-terminal-text` integration tests.

#![allow(dead_code)]

pub mod terminal;
pub mod ts;

use std::future::Future;
use std::pin::Pin;

use duplex_core::{Channel, ChannelOptions};
use duplex_test_support::future::poll;
use duplex_test_support::{Memory, MemoryQueue};
use greyhack_terminal_text::Renderer;
use terminal::{Render, RichText, Terminal};

/// Memory-connected terminal and renderer with a driven serving future.
pub struct Harness {
    queue: MemoryQueue,
    pub terminal: Channel<Terminal, Memory>,
    serve: Pin<Box<dyn Future<Output = ()>>>,
}

impl Harness {
    pub fn new() -> Self {
        let (terminal_transport, renderer_transport, queue) = Memory::transports();
        let serve = Box::pin(Renderer::new(&renderer_transport).serve());
        let terminal = Channel::new(&terminal_transport, ChannelOptions::default());

        Self {
            queue,
            terminal,
            serve,
        }
    }

    /// Sends all `(text, columns)` pairs in one render request and returns the renderer's ANSI strings.
    pub fn render(&mut self, texts: &[(&str, u16)]) -> Vec<String> {
        let pending = self
            .terminal
            .send(&Render {
                texts: texts
                    .iter()
                    .map(|&(text, columns)| RichText {
                        text: text.into(),
                        columns,
                    })
                    .collect(),
            })
            .expect("the request is sent");
        let serve = &mut self.serve;
        let rendering = self
            .queue
            .drive_with(pending, || {
                let _ = poll(serve);
            })
            .expect("the renderer responds");

        rendering.texts
    }
}
