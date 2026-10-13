use duplex_core::{Driver, OutboundBatch, Port, PortSender, Transport, WeakTransport};
use scheduler::TokioScheduler;
use tokio::task::{self, JoinHandle};

/// Drives a channel spawned by another transport on the current thread through its `Port`.
pub struct PortDriver {
    sender: PortSender,
    read_task: JoinHandle<()>,
    scheduler: TokioScheduler,
}

impl PortDriver {
    /// Creates a `port` transport using the current Tokio `LocalSet`.
    #[must_use]
    pub fn transport(port: Port) -> Transport<Self> {
        Transport::new(|transport| Self::new(port, transport))
    }

    fn new(port: Port, transport: WeakTransport<Self>) -> Self {
        let (sender, mut receiver) = port.split();
        let read_task = task::spawn_local(async move {
            while let Some(batch) = receiver.receive().await {
                transport.receive(batch);
            }

            transport.on_disconnect();
        });

        Self {
            sender,
            read_task,
            scheduler: TokioScheduler::new(),
        }
    }
}

impl Driver for PortDriver {
    type Scheduler = TokioScheduler;

    const CAPABILITIES: duplex_core::Capabilities = duplex_core::Capabilities::DUPLEX;

    fn transport_name(&self) -> &'static str {
        "port"
    }

    fn send(&self, batch: OutboundBatch) {
        let _ = self.sender.send(batch.as_slice());
    }

    fn scheduler(&self) -> &TokioScheduler {
        &self.scheduler
    }

    fn random(&self) -> [u8; 16] {
        duplex_tokio::random_bytes()
    }
}

impl Drop for PortDriver {
    fn drop(&mut self) {
        self.read_task.abort();
    }
}
