//! Process terminal hosting, game-style output, and interactive debugging.

mod debugger;
mod host;
mod screen;

pub use debugger::Debugger;
pub use host::TerminalHost;
pub use screen::RichTextScreen;
