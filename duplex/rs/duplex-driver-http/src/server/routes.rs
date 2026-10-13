use std::convert::Infallible;
use std::fmt::Write;
use std::rc::Rc;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bytes::Bytes;
use duplex_core::BatchStream;
use http_body_util::{BodyExt, Limited};
use hyper::body::Incoming;
use hyper::header::{self, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};
use tokio::sync::mpsc;
use tokio::task;

use super::Server;
use crate::client::{Backlog, Client, ClientRequest, TRANSPORT_HEADER, TransportClaim};
use crate::downstream::{Downstream, Mode, ResponseBody};

const EVENTS: &str = "/events";
const STREAM: &str = "/stream";
const EVENT_STREAM: &str = "text/event-stream";
const BATCHES: &str = "application/octet-stream";
const EVENTS_PREAMBLE: &str = ": open\n\n";
const EVENTS_HEARTBEAT: &str = ": alive\n\n";
const STREAM_HEARTBEAT: &[u8] = &[0];
/// Maximum POST-handler scheduler yields for producing downstream batches before a long-poll response.
const MOST_TURNS_TO_RESPOND: usize = 4;

enum Route {
    Post,
    Poll,
    Events,
    Stream,
    Unknown,
}

/// Stream framing with optional preamble, batch encoder, and heartbeat payload.
struct StreamFormat {
    preamble: Option<&'static [u8]>,
    after: fn(&Backlog, u64) -> Option<(Bytes, u64)>,
    heartbeat: &'static [u8],
}

impl<M: Downstream> Server<M> {
    pub(crate) async fn handle(
        self: Rc<Self>,
        request: Request<Incoming>,
    ) -> Result<Response<ResponseBody>, Infallible> {
        let preflight = request.method() == Method::OPTIONS;
        let mut response = if preflight {
            Self::status(StatusCode::NO_CONTENT)
        } else {
            self.route(request).await
        };

        self.cors.apply(&mut response, preflight);

        Ok(response)
    }

    async fn route(&self, request: Request<Incoming>) -> Response<ResponseBody> {
        let client = ClientRequest::of(request.headers());

        match self.route_of(&request) {
            Route::Post => self.post(request, client).await,
            Route::Poll => self.poll(client).await,
            Route::Events => self.events(client),
            Route::Stream => self.stream(client),
            Route::Unknown => Self::status(StatusCode::NOT_FOUND),
        }
    }

    fn route_of(&self, request: &Request<Incoming>) -> Route {
        let path = request.uri().path();
        let base = self.options.path.as_str();
        let below = path.strip_prefix(base);

        match (request.method(), M::MODE) {
            (&Method::POST, _) if path == base => Route::Post,
            (&Method::GET, Mode::LongPoll) if path == base => Route::Poll,
            (&Method::GET, Mode::EventStream) if below == Some(EVENTS) => Route::Events,
            (&Method::GET, Mode::ByteStream) if below == Some(STREAM) => Route::Stream,
            _ => Route::Unknown,
        }
    }

    async fn post(
        &self,
        request: Request<Incoming>,
        client_request: ClientRequest,
    ) -> Response<ResponseBody> {
        let named_client = match client_request.transport {
            TransportClaim::Nothing => None,
            _ => match self.client_of(&client_request) {
                Ok(client) => Some(client),
                Err(status) => return Self::status(status),
            },
        };
        let body = match Limited::new(request.into_body(), self.options.max_body)
            .collect()
            .await
        {
            Ok(body) => body.to_bytes(),
            Err(_) => return Self::status(StatusCode::PAYLOAD_TOO_LARGE),
        };
        let (client, new_transport_id) = match named_client {
            Some(client) => (client, None),
            None if body.is_empty() => return Self::status(StatusCode::BAD_REQUEST),
            None => match self.open() {
                Ok((id, client)) => (client, Some(id)),
                Err(status) => return Self::status(status),
            },
        };
        let mut response = self
            .post_batches(&client, &body, client_request.last_batch)
            .await;

        if let Some(id) = new_transport_id {
            response
                .headers_mut()
                .insert(TRANSPORT_HEADER, id.to_header());
        }

        response
    }

    /// Passes posted batches to the transport. Long-poll POST responses include batches produced within a bounded
    /// number of scheduler yields.
    async fn post_batches(
        &self,
        client: &Client,
        body: &[u8],
        last_batch: u64,
    ) -> Response<ResponseBody> {
        let _post_guard = client.begin_post();

        client.note_only(last_batch);
        client.read_gate.wait().await;

        let last_before = client.backlog().last();
        let batches = self.buffers.borrow_mut().copy(body);

        if BatchStream::split(&batches, |batch| client.receive(batch)).is_err() {
            return Self::status(StatusCode::BAD_REQUEST);
        }

        if M::MODE != Mode::LongPoll {
            return Self::status(StatusCode::NO_CONTENT);
        }

        for _ in 0..MOST_TURNS_TO_RESPOND {
            task::yield_now().await;
            if client.backlog().has_after(last_before) {
                break;
            }
        }

        match client.deliver_after(last_batch) {
            Some(records) => Self::batches(records),
            None => Self::status(StatusCode::NO_CONTENT),
        }
    }

    async fn poll(&self, request: ClientRequest) -> Response<ResponseBody> {
        let client = match self.client_of(&request) {
            Ok(client) => client,
            Err(status) => return Self::status(status),
        };
        let after = client
            .poll_after(request.last_batch, self.options.poll_timeout)
            .await;

        client.on_request();

        match Self::records_after(&client.backlog(), after) {
            Some((records, _)) => Self::batches(records),
            None => Self::status(StatusCode::NO_CONTENT),
        }
    }

    fn events(&self, request: ClientRequest) -> Response<ResponseBody> {
        let format = StreamFormat {
            preamble: Some(EVENTS_PREAMBLE.as_bytes()),
            after: Self::events_after,
            heartbeat: EVENTS_HEARTBEAT.as_bytes(),
        };

        self.stream_response(&request, format, EVENT_STREAM, "no-cache")
    }

    fn stream(&self, request: ClientRequest) -> Response<ResponseBody> {
        let format = StreamFormat {
            preamble: None,
            after: Self::records_after,
            heartbeat: STREAM_HEARTBEAT,
        };

        self.stream_response(&request, format, BATCHES, "no-cache, no-transform")
    }

    /// Creates a streaming response for batches after the client's last acknowledged sequence using `format`.
    fn stream_response(
        &self,
        request: &ClientRequest,
        format: StreamFormat,
        content_type: &'static str,
        cache_control: &'static str,
    ) -> Response<ResponseBody> {
        let client = match self.client_of(request) {
            Ok(client) => client,
            Err(status) => return Self::status(status),
        };
        let (sender, body) = mpsc::channel(1);

        task::spawn_local(Self::send_down(
            client,
            request.last_batch,
            self.options.heartbeat,
            format,
            sender,
        ));

        let mut response = Response::new(ResponseBody::Stream(body));
        let headers = response.headers_mut();

        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(cache_control),
        );

        response
    }

    async fn send_down(
        client: Rc<Client>,
        mut last_batch: u64,
        heartbeat: Duration,
        format: StreamFormat,
        sender: mpsc::Sender<Bytes>,
    ) {
        let _stream_guard = client.begin_stream();

        if let Some(preamble) = format.preamble
            && sender.send(Bytes::from_static(preamble)).await.is_err()
        {
            return;
        }

        let mut ticks = Client::heartbeat(heartbeat);

        loop {
            client.stream_after(last_batch, &mut ticks).await;

            let next = (format.after)(&client.backlog(), last_batch);
            let chunk = match next {
                Some((chunk, last)) => {
                    last_batch = last;

                    chunk
                }
                None => Bytes::from_static(format.heartbeat),
            };

            if sender.send(chunk).await.is_err() {
                return;
            }
        }
    }

    fn events_after(backlog: &Backlog, last_batch: u64) -> Option<(Bytes, u64)> {
        let mut events = String::new();
        let last = backlog.each_after(last_batch, |sequence, batch| {
            let _ = write!(events, "id: {sequence}\ndata: ");

            STANDARD.encode_string(batch, &mut events);
            events.push_str("\n\n");
        })?;

        Some((Bytes::from(events), last))
    }

    fn records_after(backlog: &Backlog, last_batch: u64) -> Option<(Bytes, u64)> {
        let (records, last) = backlog.records_after(last_batch)?;

        Some((Bytes::copy_from_slice(records), last))
    }

    /// Returns the `request` client after acknowledging reported batches, or an HTTP error status.
    fn client_of(&self, request: &ClientRequest) -> Result<Rc<Client>, StatusCode> {
        let TransportClaim::Transport(id) = request.transport else {
            return Err(StatusCode::BAD_REQUEST);
        };
        let client = self.find(id).ok_or(StatusCode::NOT_FOUND)?;

        if request.acknowledges {
            client.acknowledge(request.last_batch);
        }

        Ok(client)
    }

    fn batches(records: Bytes) -> Response<ResponseBody> {
        let mut response = Response::new(ResponseBody::full(records));

        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(BATCHES));

        response
    }

    fn status(status: StatusCode) -> Response<ResponseBody> {
        let mut response = Response::new(ResponseBody::empty());

        *response.status_mut() = status;

        response
    }
}
