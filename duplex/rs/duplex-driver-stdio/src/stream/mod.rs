#[cfg(unix)]
mod stdio;

use duplex_core::{BatchStream, Capabilities, Driver, OutboundBatch, Transport, WeakTransport};
use duplex_tokio::ReadGate;
use scheduler::TokioScheduler;
#[cfg(unix)]
use stdio::{SavedFlags, Stdio};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::{self, JoinHandle};

/// Tokio byte-stream driver with varuint-length-prefixed batches.
pub struct StreamDriver {
    name: &'static str,
    batches: UnboundedSender<OutboundBatch>,
    read_task: JoinHandle<()>,
    write_task: JoinHandle<()>,
    scheduler: TokioScheduler,
    gate: ReadGate,
    #[cfg(unix)]
    restored_flags: Option<SavedFlags>,
}

impl StreamDriver {
    /// Creates a `stream` transport over `reader` and `writer` using the current Tokio `LocalSet`.
    #[must_use]
    pub fn transport(
        reader: impl AsyncRead + Unpin + 'static,
        writer: impl AsyncWrite + Unpin + 'static,
    ) -> Transport<Self> {
        Self::transport_with_name("stream", reader, writer)
    }

    /// Calls [`StreamDriver::transport`] with `name` for protocol flavor selection.
    #[must_use]
    pub fn transport_with_name(
        name: &'static str,
        reader: impl AsyncRead + Unpin + 'static,
        writer: impl AsyncWrite + Unpin + 'static,
    ) -> Transport<Self> {
        Transport::new(|transport| Self::new(name, reader, writer, transport))
    }

    /// Creates a `stdio` transport on standard input and output.
    ///
    /// On Unix, registers pipe and socket descriptors with Tokio's reactor and restores their original flags on drop.
    #[must_use]
    pub fn stdio() -> Transport<Self> {
        Transport::new(|transport| {
            #[cfg(unix)]
            {
                let stdio = Stdio::open();
                let mut driver = Self::new("stdio", stdio.reader, stdio.writer, transport);

                driver.restored_flags = stdio.restored_flags;

                driver
            }

            #[cfg(not(unix))]
            Self::new("stdio", tokio::io::stdin(), tokio::io::stdout(), transport)
        })
    }

    /// Creates a `tcp` transport with `TCP_NODELAY` enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if `TCP_NODELAY` cannot be enabled.
    pub fn tcp(stream: TcpStream) -> std::io::Result<Transport<Self>> {
        stream.set_nodelay(true)?;

        let (reader, writer) = stream.into_split();

        Ok(Self::transport_with_name("tcp", reader, writer))
    }

    /// Calls [`StreamDriver::transport`] with transport `security`.
    #[cfg(feature = "security")]
    #[must_use]
    pub fn transport_with_security(
        reader: impl AsyncRead + Unpin + 'static,
        writer: impl AsyncWrite + Unpin + 'static,
        security: duplex_core::Security,
    ) -> Transport<Self> {
        Transport::with_security(
            |transport| Self::new("stream", reader, writer, transport),
            security,
        )
    }

    fn new(
        name: &'static str,
        reader: impl AsyncRead + Unpin + 'static,
        writer: impl AsyncWrite + Unpin + 'static,
        transport: WeakTransport<Self>,
    ) -> Self {
        let (batches, batch_rx) = mpsc::unbounded_channel();
        let gate = ReadGate::default();

        Self {
            name,
            batches,
            read_task: task::spawn_local(Self::read(reader, gate.clone(), transport.clone())),
            write_task: task::spawn_local(Self::write(writer, batch_rx, transport)),
            scheduler: TokioScheduler::new(),
            gate,
            #[cfg(unix)]
            restored_flags: None,
        }
    }

    async fn read(
        mut reader: impl AsyncRead + Unpin,
        gate: ReadGate,
        transport: WeakTransport<Self>,
    ) {
        let mut stream = BatchStream::default();

        loop {
            gate.wait().await;

            let data = match reader.read(stream.take_remains()).await {
                Ok(0) | Err(_) => break,
                Ok(data) => data,
            };

            if stream.read(data, |batch| transport.receive(batch)).is_err() {
                break;
            }
        }

        transport.on_disconnect();
    }

    async fn write(
        mut writer: impl AsyncWrite + Unpin,
        mut batches: UnboundedReceiver<OutboundBatch>,
        transport: WeakTransport<Self>,
    ) {
        while let Some(batch) = batches.recv().await {
            if Self::write_queue(&mut writer, batch, &mut batches)
                .await
                .is_err()
            {
                transport.on_disconnect();

                return;
            }
        }
    }

    async fn write_queue(
        writer: &mut (impl AsyncWrite + Unpin),
        mut batch: OutboundBatch,
        batches: &mut UnboundedReceiver<OutboundBatch>,
    ) -> std::io::Result<()> {
        loop {
            writer.write_all(batch.as_slice()).await?;

            match batches.try_recv() {
                Ok(next) => batch = next,
                Err(_) => return writer.flush().await,
            }
        }
    }
}

impl Driver for StreamDriver {
    type Scheduler = TokioScheduler;

    const CAPABILITIES: Capabilities = Capabilities::DUPLEX;
    const RESERVED_PREFIX_BYTES: usize = BatchStream::RESERVED_PREFIX_BYTES;

    fn transport_name(&self) -> &'static str {
        self.name
    }

    fn send(&self, mut batch: OutboundBatch) {
        BatchStream::frame(&mut batch);

        let _ = self.batches.send(batch);
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

impl Drop for StreamDriver {
    fn drop(&mut self) {
        self.read_task.abort();
        self.write_task.abort();
    }
}
