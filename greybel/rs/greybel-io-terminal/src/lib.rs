use std::rc::Rc;

mod keyboard;
mod screen;
mod stdin;

pub use screen::TerminalScreen;
pub use stdin::TerminalStdin;

/// Buffered terminal stdout flushed on the program clock.
pub type TerminalStdout = greybel_io::BufferedStdout<TerminalScreen, Rc<dyn scheduler::Clock>>;
