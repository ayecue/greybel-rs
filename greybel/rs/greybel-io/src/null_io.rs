use std::future::ready;
use std::io;

use crate::stdin::{InputFuture, Stdin};
use crate::stdout::Stdout;
use crate::types::{Echo, Key, PrintOptions};

#[derive(Debug, Clone, Copy, Default)]
pub struct NullIo;

impl Stdout for NullIo {
    fn print(&self, _text: &str, _options: PrintOptions) {}

    fn clear(&self) {}
}

impl Stdin for NullIo {
    fn read_line(&self, _prompt: &str, _echo: Echo) -> InputFuture<String> {
        Box::pin(ready(Err(io::ErrorKind::Unsupported.into())))
    }

    fn read_key(&self, _prompt: &str) -> InputFuture<Key> {
        Box::pin(ready(Err(io::ErrorKind::Unsupported.into())))
    }
}
