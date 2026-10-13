//! Shared helpers for `greybel-api-reference` integration tests.

#![allow(dead_code)]

pub mod guide;
pub mod reader;
pub mod reference;
pub mod search;
pub mod ts;

use std::future::Future;
use std::pin::Pin;

use duplex_core::{Channel, ChannelOptions};
use duplex_test_support::future::poll;
use duplex_test_support::{Memory, MemoryQueue};
use greybel_api_reference::Documentation;
use reader::{Block, Overview, Reader, Reference, Render, Search, SearchResult, Section};

/// In-memory reader and documentation pair with the server's future.
pub struct Harness {
    queue: MemoryQueue,
    pub reader: Channel<Reader, Memory>,
    serve: Pin<Box<dyn Future<Output = ()>>>,
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

impl Harness {
    pub fn new() -> Self {
        let (reader_transport, documentation_transport, queue) = Memory::transports();
        let serve = Box::pin(Documentation::new(&documentation_transport).serve());
        let reader = Channel::new(&reader_transport, ChannelOptions::default());

        Self {
            queue,
            reader,
            serve,
        }
    }

    /// Drives `future` while delivering memory batches and polling the documentation server until it is ready.
    pub fn wait<F: Future>(&mut self, future: F) -> F::Output {
        let serve = &mut self.serve;

        self.queue.drive_with(future, || {
            let _ = poll(serve);
        })
    }

    /// Requests the reference and returns the streamed overview and sections.
    pub fn reference(&mut self) -> (Overview, Vec<Section>) {
        let mut stream = self
            .reader
            .send(&Reference {})
            .expect("the request is sent");

        self.wait(async move {
            let overview = stream.start().await.expect("the overview arrives");
            let mut sections = Vec::new();

            while let Some(section) = stream.next().await {
                sections.push(section.expect("a section arrives"));
            }

            (overview, sections)
        })
    }

    /// Searches the API for `query`, excluding members with any of `excluded_tags`.
    pub fn search(&mut self, query: &str, excluded_tags: &[&str]) -> SearchResult {
        let pending = self
            .reader
            .send(&Search {
                query: query.to_owned().into(),
                excluded_tags: excluded_tags
                    .iter()
                    .map(|tag| (*tag).to_owned())
                    .collect::<Vec<_>>()
                    .into(),
            })
            .expect("the search is sent");

        self.wait(pending).expect("the search is answered")
    }

    /// Renders the page's `markdown`.
    pub fn render(&mut self, markdown: &str) -> Vec<Block> {
        let pending = self
            .reader
            .send(&Render {
                markdown: markdown.to_owned().into(),
            })
            .expect("the render is sent");

        self.wait(pending).expect("the render is answered")
    }
}
