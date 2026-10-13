use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use tokio::sync::mpsc::Receiver;

/// HTTP response body containing one optional chunk or a streamed chunk receiver.
pub enum ResponseBody {
    /// One optional chunk.
    Full(Option<Bytes>),
    /// Asynchronous chunks with bounded-channel sender backpressure.
    Stream(Receiver<Bytes>),
}

impl ResponseBody {
    pub(crate) fn empty() -> Self {
        Self::Full(None)
    }

    pub(crate) fn full(bytes: impl Into<Bytes>) -> Self {
        Self::Full(Some(bytes.into()))
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        match self.get_mut() {
            Self::Full(bytes) => Poll::Ready(bytes.take().map(|bytes| Ok(Frame::data(bytes)))),
            Self::Stream(chunks) => chunks
                .poll_recv(context)
                .map(|chunk| chunk.map(|chunk| Ok(Frame::data(chunk)))),
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self, Self::Full(None))
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Full(bytes) => {
                SizeHint::with_exact(bytes.as_ref().map_or(0, |bytes| bytes.len() as u64))
            }
            Self::Stream(_) => SizeHint::default(),
        }
    }
}
