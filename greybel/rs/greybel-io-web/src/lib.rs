pub mod protocol;
mod screen;
mod stdin;

use duplex_driver_web::PostMessage;
pub use screen::PageScreen;
pub use stdin::PageStdin;

/// Buffered page stdout flushed on the channel scheduler.
pub type PageStdout<D = PostMessage> =
    greybel_io::BufferedStdout<PageScreen<D>, <D as duplex_core::Driver>::Scheduler>;
