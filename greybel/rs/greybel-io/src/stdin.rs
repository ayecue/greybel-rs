use std::future::Future;
use std::io;
use std::pin::Pin;

use crate::types::{Echo, Key};

pub type InputFuture<T> = Pin<Box<dyn Future<Output = io::Result<T>>>>;

pub trait Stdin {
    fn read_line(&self, prompt: &str, echo: Echo) -> InputFuture<String>;

    fn read_key(&self, prompt: &str) -> InputFuture<Key>;

    /// Returns an available key immediately, or `None` without waiting.
    fn try_read_key(&self, prompt: &str) -> InputFuture<Option<Key>> {
        let _ = prompt;

        Box::pin(async { Ok(None) })
    }
}
