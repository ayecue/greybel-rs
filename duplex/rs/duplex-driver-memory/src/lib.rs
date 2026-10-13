//! Same-thread in-memory `duplex-core` transport pairs. [`MemoryDriver::pair`] creates connected transports and
//! delivers each batch unchanged through a deferred scheduler task. The supplied [`scheduler::Scheduler`] provides
//! tasks, timers, and time from another transport (`driver().scheduler().clone()`), Tokio, or a browser.

#![warn(missing_docs)]
mod driver;

pub use driver::MemoryDriver;
