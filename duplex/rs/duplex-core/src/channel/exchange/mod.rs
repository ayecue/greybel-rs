mod io;
mod reader;
mod writer;

use std::marker::PhantomData;

pub(crate) use io::{ChannelIo, ItemSource};
pub use io::{IncomingExchange, OutgoingExchange};
pub use reader::{IncomingStream, PendingResponse, ResponseStream};
pub use writer::{ItemWriter, Responder, StreamResponder, StreamWriter};

type TypeMarker<T> = PhantomData<fn() -> T>;
