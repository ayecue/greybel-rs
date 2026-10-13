use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;

use duplex_core::{Channel, ChannelOptions, ErrorCode};
use duplex_driver_stdio::StreamDriver;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::task::LocalSet;

use super::hub::Hub;
use super::{ClientError, ClientOptions};
use crate::protocol::GreybelNotification;
use crate::protocol::greyhack::Greybel;

/// Send operation run on the connection thread with the client's protocol channel.
pub(crate) type Outbound =
    Box<dyn FnOnce(&Channel<Greybel, StreamDriver>) -> Result<(), duplex_core::Error> + Send>;

type Delivery = (Outbound, oneshot::Sender<Result<(), ClientError>>);

/// Owns the plugin duplex channel's dedicated thread and Tokio `LocalSet`. Runs outbound closures on that thread
/// and routes incoming notifications to the client hub.
#[derive(Debug)]
pub(crate) struct Connection {
    requests: mpsc::UnboundedSender<Delivery>,
}

impl Connection {
    /// Spawns the channel thread, retries TCP as configured by `options`, and awaits protocol `Ack`.
    pub(crate) async fn connect(
        options: &ClientOptions,
        hub: Arc<Hub>,
    ) -> Result<Self, ClientError> {
        let (ready, acknowledgement) = oneshot::channel();
        let (requests, deliveries) = mpsc::unbounded_channel();
        let options = options.clone();

        thread::Builder::new()
            .name("greyhack message hook".to_owned())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready.send(Err(ClientError::Io(error)));

                        hub.close();

                        return;
                    }
                };

                LocalSet::new()
                    .block_on(&runtime, run(options, Arc::clone(&hub), deliveries, ready));
                hub.close();
            })?;
        acknowledgement
            .await
            .map_err(|_| ClientError::Disconnect)??;

        Ok(Self { requests })
    }

    /// Enqueues an outbound closure on the channel thread and awaits the send result.
    pub(crate) async fn send(&self, outbound: Outbound) -> Result<(), ClientError> {
        let (reply, outcome) = oneshot::channel();

        self.requests
            .send((outbound, reply))
            .map_err(|_| ClientError::Disconnect)?;

        outcome.await.map_err(|_| ClientError::Disconnect)?
    }
}

async fn connect(options: &ClientOptions) -> Result<TcpStream, ClientError> {
    let addr = format!("{}:{}", options.host, options.port);
    let attempts = options.connect_attempts.max(1);
    let mut attempt = 1;

    loop {
        match TcpStream::connect(&addr).await {
            Ok(stream) => return Ok(stream),
            Err(source) if attempt >= attempts => {
                return Err(ClientError::Connect {
                    addr,
                    attempts,
                    source,
                });
            }
            Err(_) => {
                attempt += 1;
                tokio::time::sleep(options.retry_delay).await;
            }
        }
    }
}

async fn run(
    options: ClientOptions,
    hub: Arc<Hub>,
    mut deliveries: mpsc::UnboundedReceiver<Delivery>,
    ready: oneshot::Sender<Result<(), ClientError>>,
) {
    let transport = match connect(&options)
        .await
        .and_then(|stream| Ok(StreamDriver::tcp(stream)?))
    {
        Ok(transport) => transport,
        Err(error) => {
            let _ = ready.send(Err(error));

            return;
        }
    };
    let channel = Channel::<Greybel, StreamDriver>::new(&transport, ChannelOptions::default());

    GreybelNotification::attach(&channel, move |notification| hub.dispatch(&notification));

    let ready = Rc::new(RefCell::new(Some(ready)));

    channel.on_ack({
        let ready = Rc::clone(&ready);

        move |_| {
            if let Some(ready) = ready.borrow_mut().take() {
                let _ = ready.send(Ok(()));
            }
        }
    });
    channel.on_failure({
        let ready = Rc::clone(&ready);

        move |failure| {
            if let Some(ready) = ready.borrow_mut().take() {
                let _ = ready.send(Err(ClientError::Refusal(failure.reason())));
            }
        }
    });
    loop {
        tokio::select! {
            item = deliveries.recv() => match item {
                Some((outbound, reply)) => {
                    let _ = reply.send(outbound(&channel).map_err(ClientError::from));
                }
                None => {
                    channel.close(ErrorCode::ClosedChannel);
                    break;
                }
            },
            _ = channel.close_event() => break,
        }
    }

    if let Some(ready) = ready.borrow_mut().take() {
        let _ = ready.send(Err(ClientError::Disconnect));
    }
}
