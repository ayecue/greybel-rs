//! Hosts Grey Hack's terminal in a browser worker, exposing the terminal view to the page and rendering program
//! output relayed through the io protocol.

pub mod protocol;
mod service;
mod terminal;
mod view;
#[cfg(target_arch = "wasm32")]
mod worker;

pub use service::TerminalService;
pub use terminal::Terminal;
pub use view::TerminalView;
