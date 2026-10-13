use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;

use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::Builder;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::{self, LocalSet};

use crate::Acceptor;

type Job = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>;

struct Worker {
    job_sender: Option<UnboundedSender<Job>>,
    thread: Option<JoinHandle<()>>,
}

/// Worker threads running one duplex hub per current-thread Tokio runtime. Jobs use round-robin assignment by
/// default.
pub struct HubManager {
    workers: Vec<Worker>,
    next_worker: AtomicUsize,
}

impl HubManager {
    /// Starts `threads` workers, or one worker if `threads` is zero.
    ///
    /// # Errors
    ///
    /// Returns an error if Tokio runtime or worker creation fails.
    pub fn start(threads: usize) -> io::Result<Self> {
        let workers = (0..threads.max(1))
            .map(Self::worker)
            .collect::<io::Result<Vec<_>>>()?;

        Ok(Self {
            workers,
            next_worker: AtomicUsize::new(0),
        })
    }

    fn worker(index: usize) -> io::Result<Worker> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let (job_sender, mut job_receiver) = mpsc::unbounded_channel::<Job>();
        let thread = std::thread::Builder::new()
            .name(format!("duplex-hub-{index}"))
            .spawn(move || {
                LocalSet::new().block_on(&runtime, async move {
                    while let Some(job) = job_receiver.recv().await {
                        task::spawn_local(job());
                    }
                });
            })?;

        Ok(Worker {
            job_sender: Some(job_sender),
            thread: Some(thread),
        })
    }

    /// Returns worker thread count.
    #[must_use]
    pub fn threads(&self) -> usize {
        self.workers.len()
    }

    /// Spawns `job` on the next round-robin worker and returns its zero-based index.
    pub fn spawn<F, Fut>(&self, job: F) -> usize
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let thread = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();

        self.spawn_on(thread, job);

        thread
    }

    /// Spawns `job` on worker `thread`.
    ///
    /// # Panics
    ///
    /// Panics when `thread` is outside the worker index range.
    pub fn spawn_on<F, Fut>(&self, thread: usize, job: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let job: Job = Box::new(move || Box::pin(job()));

        if let Some(job_sender) = &self.workers[thread].job_sender {
            let _ = job_sender.send(job);
        }
    }

    /// Accepts TCP connections and assigns them round-robin. `serve` handles each connection's transport and channels.
    /// Accept errors use [`Acceptor`] retry behavior; this function does not return.
    pub async fn serve_tcp<F, Fut>(&self, listener: TcpListener, serve: F)
    where
        F: Fn(TcpStream) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let acceptor = Acceptor::new(listener);

        loop {
            let (stream, _) = acceptor.accept().await;
            let Ok(stream) = stream.into_std() else {
                continue;
            };
            let serve = serve.clone();

            self.spawn(move || async move {
                if let Ok(stream) = TcpStream::from_std(stream) {
                    serve(stream).await;
                }
            });
        }
    }
}

impl Drop for HubManager {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            worker.job_sender.take();
        }

        let current = std::thread::current().id();

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take()
                && thread.thread().id() != current
            {
                let _ = thread.join();
            }
        }
    }
}
