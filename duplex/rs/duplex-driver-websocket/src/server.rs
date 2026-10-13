use std::io;
use std::net::SocketAddr;
use std::rc::Rc;

use duplex_core::Transport;
use duplex_tokio::Acceptor;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::{self, JoinHandle};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{StatusCode, header};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

use crate::driver::WebSocketDriver;
use crate::options::WebSocketOptions;

/// Per-connection read buffer bytes. Larger messages require multiple reads.
const READ_BUFFER_BYTES: usize = 16 * 1024;
/// Tungstenite write buffer bytes. The driver flushes after each queued batch run.
const WRITE_BUFFER_BYTES: usize = 8 * 1024;

/// Accepts WebSocket clients and returns their transports through [`WebSocketServer::accept`].
pub struct WebSocketServer {
    address: SocketAddr,
    transport_rx: UnboundedReceiver<Transport<WebSocketDriver>>,
    listen_task: JoinHandle<()>,
}

impl WebSocketServer {
    /// # Errors
    ///
    /// Returns an error if TCP binding to `address` or querying the local address fails.
    pub async fn bind(address: impl ToSocketAddrs, options: WebSocketOptions) -> io::Result<Self> {
        let listener = TcpListener::bind(address).await?;
        let address = listener.local_addr()?;
        let (transport_tx, transports) = mpsc::unbounded_channel();
        let listen_task = task::spawn_local(Self::listen(listener, Rc::new(options), transport_tx));

        Ok(Self {
            address,
            transport_rx: transports,
            listen_task,
        })
    }

    /// Returns the server's bound socket address.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// Waits for the next client transport.
    pub async fn accept(&mut self) -> Option<Transport<WebSocketDriver>> {
        self.transport_rx.recv().await
    }

    async fn listen(
        listener: TcpListener,
        options: Rc<WebSocketOptions>,
        transport_tx: UnboundedSender<Transport<WebSocketDriver>>,
    ) {
        let acceptor = Acceptor::new(listener);

        loop {
            let (stream, _) = acceptor.accept().await;

            task::spawn_local(Self::open(
                stream,
                Rc::clone(&options),
                transport_tx.clone(),
            ));
        }
    }

    async fn open(
        stream: TcpStream,
        options: Rc<WebSocketOptions>,
        transport_tx: UnboundedSender<Transport<WebSocketDriver>>,
    ) {
        if let Some(transport) = Self::handshake(stream, &options).await {
            let _ = transport_tx.send(transport);
        }
    }

    /// Creates a client transport from `stream` when its WebSocket handshake satisfies `options`.
    /// Accepts streams delegated to the current thread by [`duplex_tokio::HubManager`].
    #[expect(
        clippy::result_large_err,
        reason = "tungstenite's handshake callback returns its error response by value"
    )]
    pub async fn handshake(
        stream: TcpStream,
        options: &WebSocketOptions,
    ) -> Option<Transport<WebSocketDriver>> {
        let _ = stream.set_nodelay(true);
        let config = WebSocketConfig::default()
            .read_buffer_size(READ_BUFFER_BYTES)
            .write_buffer_size(WRITE_BUFFER_BYTES)
            .max_message_size(Some(options.max_batch))
            .max_frame_size(Some(options.max_batch));
        let check = move |request: &Request, response: Response| {
            Self::check(options, request)
                .map(|()| response)
                .map_err(Self::refusal)
        };
        let socket = tokio_tungstenite::accept_hdr_async_with_config(stream, check, Some(config))
            .await
            .ok()?;
        let max_pending_bytes = options.max_pending_bytes;

        Some(Transport::new(|weak| {
            WebSocketDriver::new(socket, max_pending_bytes, weak)
        }))
    }

    fn check(options: &WebSocketOptions, request: &Request) -> Result<(), StatusCode> {
        if request.uri().path() != options.path {
            return Err(StatusCode::NOT_FOUND);
        }

        let header = |name| {
            request
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
        };

        if !options.allows(header(header::ORIGIN), header(header::HOST)) {
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(())
    }

    fn refusal(status: StatusCode) -> ErrorResponse {
        let mut response = ErrorResponse::new(None);

        *response.status_mut() = status;

        response
    }
}

impl Drop for WebSocketServer {
    fn drop(&mut self) {
        self.listen_task.abort();
    }
}
