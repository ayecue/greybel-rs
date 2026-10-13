use std::borrow::Cow;

use duplex_core::{Channel, Driver};
use duplex_driver_web::PostMessage;
use greybel_io::{Output, Screen};

use crate::protocol::{OutputBatch, Program};

pub struct PageScreen<D: Driver = PostMessage> {
    channel: Channel<Program, D>,
}

impl<D: Driver> PageScreen<D> {
    #[must_use]
    pub fn new(channel: &Channel<Program, D>) -> Self {
        Self {
            channel: channel.clone(),
        }
    }
}

impl<D: Driver> Screen for PageScreen<D> {
    fn show(&self, batch: &[Output]) {
        let _ = self.channel.send(&OutputBatch {
            entries: Cow::Borrowed(batch),
        });
    }
}
