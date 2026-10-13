use std::cell::Cell;
use std::rc::Rc;

use duplex_core::{Capabilities, Driver, InboundBufferManager, OutboundBatch, WeakTransport};
use duplex_tokio::ReadGate;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use scheduler::{Scheduler, TokioScheduler};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::{self, JoinHandle};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::{Bytes, Message};

type Socket = WebSocketStream<TcpStream>;

/// Minimum batch size for moving the outbound buffer into a WebSocket message without copying. Smaller batches are
/// copied.
const ZERO_COPY_MIN: usize = 16 * 1024;

/// Provider WebSocket driver for one client transport. Sends each batch as one binary WebSocket message.
pub struct WebSocketDriver {
    batches: UnboundedSender<Bytes>,
    pending_bytes: Rc<Cell<usize>>,
    max_pending_bytes: usize,
    transport: WeakTransport<Self>,
    read_task: JoinHandle<()>,
    write_task: JoinHandle<()>,
    scheduler: TokioScheduler,
    gate: ReadGate,
}

impl WebSocketDriver {
    pub(crate) fn new(
        socket: Socket,
        max_pending_bytes: usize,
        transport: WeakTransport<Self>,
    ) -> Self {
        let (sink, stream) = socket.split();
        let (batches, batch_rx) = mpsc::unbounded_channel();
        let pending_bytes = Rc::new(Cell::new(0));
        let gate = ReadGate::default();

        Self {
            batches,
            max_pending_bytes,
            read_task: task::spawn_local(Self::read(stream, gate.clone(), transport.clone())),
            write_task: task::spawn_local(Self::write(
                sink,
                batch_rx,
                Rc::clone(&pending_bytes),
                transport.clone(),
            )),
            pending_bytes,
            transport,
            scheduler: TokioScheduler::new(),
            gate,
        }
    }

    async fn read(mut stream: SplitStream<Socket>, gate: ReadGate, transport: WeakTransport<Self>) {
        let mut buffers = InboundBufferManager::default();

        loop {
            gate.wait().await;

            let Some(message) = stream.next().await else {
                break;
            };

            match message {
                Ok(Message::Binary(batch)) => transport.receive(buffers.copy(&batch)),
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }

        transport.on_disconnect();
    }

    async fn write(
        mut sink: SplitSink<Socket, Message>,
        mut batches: UnboundedReceiver<Bytes>,
        pending_bytes: Rc<Cell<usize>>,
        transport: WeakTransport<Self>,
    ) {
        while let Some(batch) = batches.recv().await {
            if Self::write_queue(&mut sink, batch, &mut batches, &pending_bytes)
                .await
                .is_err()
            {
                transport.on_disconnect();

                return;
            }
        }
    }

    async fn write_queue(
        sink: &mut SplitSink<Socket, Message>,
        mut batch: Bytes,
        batches: &mut UnboundedReceiver<Bytes>,
        pending_bytes: &Cell<usize>,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        loop {
            pending_bytes.set(pending_bytes.get().saturating_sub(batch.len()));
            sink.feed(Message::Binary(batch)).await?;

            match batches.try_recv() {
                Ok(next) => batch = next,
                Err(_) => return sink.flush().await,
            }
        }
    }
}

impl Driver for WebSocketDriver {
    type Scheduler = TokioScheduler;

    const CAPABILITIES: Capabilities = Capabilities::DUPLEX.without(Capabilities::CLIENT);

    fn transport_name(&self) -> &'static str {
        "websocket"
    }

    fn send(&self, batch: OutboundBatch) {
        let pending_total = self.pending_bytes.get() + batch.len();

        if pending_total > self.max_pending_bytes {
            self.write_task.abort();

            let transport = self.transport.clone();

            self.scheduler
                .defer(Box::new(move || transport.on_disconnect()));

            return;
        }

        self.pending_bytes.set(pending_total);

        let bytes = if batch.len() < ZERO_COPY_MIN {
            Bytes::copy_from_slice(batch.as_slice())
        } else {
            Bytes::from(batch.into_vec())
        };
        let _ = self.batches.send(bytes);
    }

    fn scheduler(&self) -> &TokioScheduler {
        &self.scheduler
    }

    fn random(&self) -> [u8; 16] {
        duplex_tokio::random_bytes()
    }

    fn pause(&self) {
        self.gate.pause();
    }

    fn resume(&self) {
        self.gate.resume();
    }
}

impl Drop for WebSocketDriver {
    fn drop(&mut self) {
        self.read_task.abort();
        self.write_task.abort();
    }
}
