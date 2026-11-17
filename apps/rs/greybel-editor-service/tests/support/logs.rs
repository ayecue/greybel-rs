//! Captures the logs of one test.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};

use super::run;

/// Code that must not appear in any log.
pub const SECRET_CODE: &str = "print \"secret token 12345\"";

thread_local! {
    static THREAD_BUFFER: RefCell<Option<Arc<Mutex<Vec<u8>>>>> = const { RefCell::new(None) };
}

/// Log lines written while `run` executes, in the text format. One global subscriber writes to the buffer of the
/// running thread: a per-thread default subscriber misses events whose callsites another test thread hit first.
#[derive(Clone, Default)]
pub struct LogCapture {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl LogCapture {
    pub fn run<F: Future>(&self, future: F) -> F::Output {
        static SUBSCRIBER: OnceLock<()> = OnceLock::new();

        SUBSCRIBER.get_or_init(|| {
            let subscriber = tracing_subscriber::fmt()
                .with_writer(|| ThreadWriter)
                .with_ansi(false)
                .with_max_level(tracing::Level::INFO)
                .finish();

            tracing::subscriber::set_global_default(subscriber).expect("one global subscriber");
        });
        THREAD_BUFFER.with(|buffer| *buffer.borrow_mut() = Some(self.bytes.clone()));

        let output = run(future);

        THREAD_BUFFER.with(|buffer| *buffer.borrow_mut() = None);

        output
    }

    /// The lines that contain `text`.
    pub fn find(&self, text: &str) -> Vec<String> {
        let bytes = self.bytes.lock().expect("the log");

        String::from_utf8_lossy(&bytes)
            .lines()
            .filter(|line| line.contains(text))
            .map(str::to_owned)
            .collect()
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes.lock().expect("the log")).into_owned()
    }
}

/// Appends to the running thread's capture buffer; discards outside `LogCapture::run`.
struct ThreadWriter;

impl std::io::Write for ThreadWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        THREAD_BUFFER.with(|capture| {
            if let Some(bytes) = capture.borrow().as_ref() {
                bytes.lock().expect("the log").extend_from_slice(buffer);
            }
        });

        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
