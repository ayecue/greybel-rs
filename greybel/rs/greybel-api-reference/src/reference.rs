//! In-process reference generation and search without duplex transport.

use crate::protocol::{Block, Overview, Render, Search, SearchResult, Section};
use crate::section::SectionWriter;

/// Builds overview and type sections equivalent to the payload streamed by [`Documentation`](crate::Documentation)
/// for a [`Reference`](crate::protocol::Reference).
#[must_use]
pub fn reference() -> (Overview, Vec<Section>) {
    let writer = SectionWriter::new();

    (Overview::new(), writer.sections())
}

/// Finds types and members matching `search` with documentation search semantics.
#[must_use]
pub fn search(search: &Search<'_>) -> SearchResult {
    SearchResult::of(search)
}

/// Converts `render`'s Markdown to blocks, matching [`Documentation`](crate::Documentation)'s response to a
/// [`Render`](crate::protocol::Render).
#[must_use]
pub fn render(render: &Render<'_>) -> Vec<Block> {
    SectionWriter::new().blocks(&render.markdown)
}
