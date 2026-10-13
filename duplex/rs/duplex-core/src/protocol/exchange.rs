use std::fmt::Debug;

use crate::binary::{ValidatedDecode, ValidatedEncode};
use crate::channel::{
    IncomingExchange, IncomingStream, OutgoingExchange, PendingResponse, Responder, ResponseStream,
    StreamResponder, StreamWriter,
};
use crate::definition::ErrorCode;

type TypeMarker<T> = std::marker::PhantomData<fn() -> T>;

#[doc(hidden)]
pub trait Exchange: 'static {
    type Handle;
    type Responder;
    const RESPONDS: bool;
    const STREAMS: bool;
    const STREAMS_RESPONSE: bool;
    const SENT_REFS: u8;
    const RESPONDER_REFS: u8;

    fn handle(exchange: Option<OutgoingExchange>) -> Self::Handle;
    fn responder(exchange: &IncomingExchange) -> Self::Responder;
    fn reject(responder: Self::Responder, code: ErrorCode);
}

#[doc(hidden)]
pub struct NoResponse;

#[doc(hidden)]
pub struct ValueResponse<R, F = std::convert::Infallible>(TypeMarker<(R, F)>);

#[doc(hidden)]
pub struct StreamResponse<S, E, I, F = std::convert::Infallible>(TypeMarker<(S, E, I, F)>);

#[doc(hidden)]
pub struct StreamExchange<E, I, X>(TypeMarker<(E, I, X)>);

impl Exchange for NoResponse {
    type Handle = ();
    type Responder = ();

    const RESPONDER_REFS: u8 = 0;
    const RESPONDS: bool = false;
    const SENT_REFS: u8 = 0;
    const STREAMS: bool = false;
    const STREAMS_RESPONSE: bool = false;

    fn handle(_: Option<OutgoingExchange>) {}

    fn responder(_: &IncomingExchange) {}

    fn reject((): (), _: ErrorCode) {}
}

impl<R, F> Exchange for ValueResponse<R, F>
where
    R: ValidatedEncode + ValidatedDecode + 'static,
    F: ValidatedEncode + ValidatedDecode + Debug + 'static,
{
    type Handle = PendingResponse<R, F>;
    type Responder = Responder<R, F>;

    const RESPONDER_REFS: u8 = 1;
    const RESPONDS: bool = true;
    const SENT_REFS: u8 = 1;
    const STREAMS: bool = false;
    const STREAMS_RESPONSE: bool = false;

    fn handle(exchange: Option<OutgoingExchange>) -> Self::Handle {
        PendingResponse::new(exchange.expect("a message with a response has an id"))
    }

    fn responder(exchange: &IncomingExchange) -> Self::Responder {
        Responder::new(exchange)
    }

    fn reject(responder: Self::Responder, code: ErrorCode) {
        responder.reject(code);
    }
}

impl<S, E, I, F> Exchange for StreamResponse<S, E, I, F>
where
    S: ValidatedEncode + ValidatedDecode + 'static,
    E: ValidatedEncode + ValidatedDecode + 'static,
    I: ValidatedEncode + ValidatedDecode + 'static,
    F: ValidatedEncode + ValidatedDecode + Debug + 'static,
{
    type Handle = ResponseStream<S, E, I, F>;
    type Responder = StreamResponder<S, E, I, F>;

    const RESPONDER_REFS: u8 = 2;
    const RESPONDS: bool = true;
    const SENT_REFS: u8 = 1;
    const STREAMS: bool = false;
    const STREAMS_RESPONSE: bool = true;

    fn handle(exchange: Option<OutgoingExchange>) -> Self::Handle {
        ResponseStream::new(exchange.expect("a message with a response has an id"))
    }

    fn responder(exchange: &IncomingExchange) -> Self::Responder {
        StreamResponder::new(exchange)
    }

    fn reject(responder: Self::Responder, code: ErrorCode) {
        responder.reject(code);
    }
}

impl<E, I, X> Exchange for StreamExchange<E, I, X>
where
    E: ValidatedEncode + ValidatedDecode + 'static,
    I: ValidatedEncode + ValidatedDecode + 'static,
    X: Exchange,
{
    type Handle = (StreamWriter<E, I>, X::Handle);
    type Responder = (IncomingStream<E, I>, X::Responder);

    const RESPONDER_REFS: u8 = 1 + X::RESPONDER_REFS;
    const RESPONDS: bool = true;
    const SENT_REFS: u8 = 1 + X::SENT_REFS;
    const STREAMS: bool = true;
    const STREAMS_RESPONSE: bool = X::STREAMS_RESPONSE;

    fn handle(exchange: Option<OutgoingExchange>) -> Self::Handle {
        let exchange = exchange.expect("a message with a stream has an id");
        let writer = StreamWriter::new(&exchange);

        (writer, X::handle(Some(exchange)))
    }

    fn responder(exchange: &IncomingExchange) -> Self::Responder {
        (IncomingStream::new(exchange), X::responder(exchange))
    }

    fn reject((_, responder): Self::Responder, code: ErrorCode) {
        X::reject(responder, code);
    }
}
