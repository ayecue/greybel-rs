//! Converts Grey Hack rich text to ANSI ([`ansi_text`]) or plain text ([`plain_text`]). The duplex [`Renderer`]
//! handles terminal ANSI-rendering requests.

pub mod protocol;
mod renderer;
mod text;

pub use renderer::Renderer;
pub use text::{ansi_text, plain_text};
