//! Builds a GreyScript API reference from metadata with Markdown descriptions, highlighted examples, and query
//! search. Page Markdown uses the same rendering. Call [`reference()`], [`search()`], and [`render()`] directly,
//! serialize with [`protocol::Section::markdown`], or serve duplex readers through [`Documentation`].

mod anchors;
mod api;
mod documentation;
mod example;
mod markdown;
mod overview;
pub mod protocol;
mod reference;
mod search;
mod section;
mod text;
mod type_text;

pub use documentation::Documentation;
pub use reference::{reference, render, search};
