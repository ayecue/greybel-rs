use std::io::{self, BufRead, Write};
use std::panic::{self, AssertUnwindSafe};

use greybel_lsp::types::ErrorCodes;
use greybel_lsp::{Message, Response, ResponseError, Server};

use crate::transport::Transport;

pub struct Driver<R, W> {
    server: Server,
    transport: Transport<R, W>,
}

impl<R: BufRead, W: Write> Driver<R, W> {
    const END_OF_INPUT_EXIT_CODE: i32 = 1;

    pub fn new(reader: R, writer: W) -> Self {
        Self {
            server: Server::with_local_files(),
            transport: Transport::new(reader, writer),
        }
    }

    pub fn run(&mut self) -> io::Result<i32> {
        while let Some(message) = self.transport.read()? {
            self.deliver(message)?;
            if let Some(code) = self.server.exit_code() {
                return Ok(code);
            }
        }

        Ok(Self::END_OF_INPUT_EXIT_CODE)
    }

    fn deliver(&mut self, message: Message) -> io::Result<()> {
        for output in self.receive(message) {
            self.transport.write(&output)?;
        }

        Ok(())
    }

    fn receive(&mut self, message: Message) -> Vec<Message> {
        let id = match &message {
            Message::Request(request) => Some(request.id.clone()),
            _ => None,
        };
        let server = &mut self.server;

        match panic::catch_unwind(AssertUnwindSafe(|| server.receive(message))) {
            Ok(output) => output,
            Err(panic) => {
                let reason = panic
                    .downcast_ref::<&str>()
                    .map(|reason| (*reason).to_owned())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_default();

                eprintln!("greybel-lsp: a message failed: {reason}");

                id.map(|id| {
                    Message::Response(Response::error(
                        Some(id),
                        ResponseError::new(ErrorCodes::InternalError, reason),
                    ))
                })
                .into_iter()
                .collect()
            }
        }
    }
}
