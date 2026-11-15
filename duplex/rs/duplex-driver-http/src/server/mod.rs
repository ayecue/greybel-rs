mod cors;
mod options;
mod routes;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use cors::Cors;
use duplex_core::{InboundBufferManager, Transport};
use duplex_tokio::Acceptor;
use hyper::StatusCode;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
pub use options::HttpOptions;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::{self, JoinHandle};

use crate::client::{Client, TransportId};
use crate::downstream::Downstream;
use crate::driver::HttpDriver;

const SWEEP: Duration = Duration::from_secs(1);

/// Accepts HTTP client transports on the current Tokio `LocalSet`; [`HttpServer::accept`] returns new transports.
pub struct HttpServer<M: Downstream> {
    address: SocketAddr,
    transport_rx: UnboundedReceiver<Transport<HttpDriver<M>>>,
    tasks: [JoinHandle<()>; 2],
}

/// Shared request state with clients indexed by transport ID and reusable POST body buffers.
pub(crate) struct Server<M: Downstream> {
    pub(crate) options: HttpOptions,
    pub(crate) cors: Cors,
    pub(crate) buffers: RefCell<InboundBufferManager>,
    clients: RefCell<HashMap<TransportId, Rc<Client>>>,
    transport_tx: UnboundedSender<Transport<HttpDriver<M>>>,
    downstream: PhantomData<fn() -> M>,
}

impl<M: Downstream> HttpServer<M> {
    /// # Errors
    ///
    /// Returns an error if TCP binding to `address` or querying the local address fails.
    pub async fn bind(address: impl ToSocketAddrs, options: HttpOptions) -> io::Result<Self> {
        let listener = TcpListener::bind(address).await?;
        let address = listener.local_addr()?;
        let (transport_tx, transports) = mpsc::unbounded_channel();
        let server = Rc::new(Server {
            cors: Cors::new(options.allow_origin.as_deref(), options.preflight_max_age),
            options,
            buffers: RefCell::default(),
            clients: RefCell::default(),
            transport_tx,
            downstream: PhantomData,
        });
        let listen_task = task::spawn_local(Server::listen(Rc::clone(&server), listener));
        let sweep_task = task::spawn_local(Server::sweep(server));

        Ok(Self {
            address,
            transport_rx: transports,
            tasks: [listen_task, sweep_task],
        })
    }

    /// Returns the server's bound socket address.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// Waits for the next client transport.
    pub async fn accept(&mut self) -> Option<Transport<HttpDriver<M>>> {
        self.transport_rx.recv().await
    }
}

impl<M: Downstream> Drop for HttpServer<M> {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

impl<M: Downstream> Server<M> {
    async fn listen(self: Rc<Self>, listener: TcpListener) {
        let acceptor = Acceptor::new(listener);

        loop {
            let (stream, _) = acceptor.accept().await;
            let _ = stream.set_nodelay(true);
            let server = Rc::clone(&self);

            task::spawn_local(async move {
                let service = service_fn(move |request| Rc::clone(&server).handle(request));
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    }

    /// Disconnects overloaded or idle clients on a one-second interval.
    async fn sweep(self: Rc<Self>) {
        let mut ticks = tokio::time::interval(SWEEP);
        let mut clients_to_disconnect = Vec::new();

        loop {
            ticks.tick().await;

            let idle = self.options.idle;

            clients_to_disconnect.extend(
                self.clients
                    .borrow_mut()
                    .extract_if(|_, client| client.should_disconnect_after(SWEEP, idle))
                    .map(|(_, client)| client),
            );
            for client in clients_to_disconnect.drain(..) {
                client.disconnect();
            }
        }
    }

    pub(crate) fn find(&self, id: TransportId) -> Option<Rc<Client>> {
        let client = Rc::clone(self.clients.borrow().get(&id)?);

        client.on_request();

        Some(client)
    }

    /// Creates and queues a client transport for [`HttpServer::accept`], or returns an HTTP error status.
    pub(crate) fn open(&self) -> Result<(TransportId, Rc<Client>), StatusCode> {
        if self.clients.borrow().len() >= self.options.max_transports {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        let id = loop {
            let id = TransportId::random().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

            if !self.clients.borrow().contains_key(&id) {
                break id;
            }
        };
        let client = Rc::new(Client::new(self.options.max_backlog));
        let transport = Transport::new(|weak| {
            let disconnect_handle = weak.clone();

            client.connect(
                Box::new(move |batch| weak.receive(batch)),
                Box::new(move || disconnect_handle.on_disconnect()),
            );

            HttpDriver::<M>::new(Rc::clone(&client))
        });

        if self.transport_tx.send(transport).is_err() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        self.clients.borrow_mut().insert(id, Rc::clone(&client));

        Ok((id, client))
    }
}
