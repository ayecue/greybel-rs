use std::borrow::Cow;
use std::io;
use std::rc::Rc;

use duplex_core::{Channel, Driver, Error};
use duplex_driver_web::PostMessage;
use greybel_io::{Echo, InputFuture, Key, ReadFailure, Stdin, Stdout};

use crate::protocol::{Program, ReadKey, ReadLine};

pub struct PageStdin<D: Driver = PostMessage> {
    channel: Channel<Program, D>,
    stdout: Rc<dyn Stdout>,
}

impl<D: Driver> PageStdin<D> {
    #[must_use]
    pub fn new(channel: &Channel<Program, D>, stdout: Rc<dyn Stdout>) -> Self {
        Self {
            channel: channel.clone(),
            stdout,
        }
    }
}

impl<D: Driver> Stdin for PageStdin<D> {
    fn read_line(&self, prompt: &str, echo: Echo) -> InputFuture<String> {
        self.stdout.flush();

        let response = self.channel.send(&ReadLine {
            prompt: Cow::Borrowed(prompt),
            echo,
        });

        Box::pin(async move {
            let response = response.map_err(|error| input_error(error.with_failure()))?;

            response.await.map_err(input_error)
        })
    }

    fn read_key(&self, prompt: &str) -> InputFuture<Key> {
        self.stdout.flush();

        let response = self.channel.send(&ReadKey {
            prompt: Cow::Borrowed(prompt),
        });

        Box::pin(async move {
            let response = response.map_err(|error| input_error(error.with_failure()))?;

            response.await.map_err(input_error)
        })
    }
}

const CANCEL_MESSAGE: &str = "the input was cancelled";

fn input_error(error: Error<ReadFailure>) -> io::Error {
    match error {
        Error::Failure(ReadFailure::Cancellation) => {
            io::Error::new(io::ErrorKind::Interrupted, CANCEL_MESSAGE)
        }
        Error::Rejection(code) => io::Error::new(io::ErrorKind::Interrupted, code.message()),
        Error::ChannelEnd(code) => io::Error::new(io::ErrorKind::BrokenPipe, code.message()),
        Error::Encode(_) | Error::Decode(_) | Error::Invalid(_) => {
            io::Error::new(io::ErrorKind::InvalidData, error)
        }
        Error::Unavailable => io::Error::new(io::ErrorKind::Unsupported, error),
    }
}
