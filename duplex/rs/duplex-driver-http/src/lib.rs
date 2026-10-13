//! `duplex-core` HTTP provider drivers with sequenced batches and retryable downstream responses.

#![warn(missing_docs)]
mod client;
mod downstream;
mod driver;
mod server;

pub use client::{RECEIVED_HEADER, TRANSPORT_HEADER};
pub use downstream::{ByteStream, Downstream, EventStream, LongPoll, Mode, ResponseBody};
pub use driver::{HTTP_STREAM_WINDOW, HttpDriver};
pub use server::{HttpOptions, HttpServer};
