//! In-memory io terminal with recorded output and queued line/key responses.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greybel_io::{Key, Output, ReadFailure};
use greybel_io_web::protocol::{OutputBatch, Page, ReadKey, ReadLine};

use crate::io::PrintLog;

/// Records output batches and returns queued line/key input. Empty input queues fail with
/// `ReadFailure::Cancellation`.
#[derive(Default)]
pub struct TerminalRecorder {
    /// Received output batches in order.
    pub batches: RefCell<Vec<Vec<Output>>>,
    /// Received PrintLog text in order.
    pub print_log: PrintLog,
    /// Pending line-read responses.
    pub lines: RefCell<VecDeque<String>>,
    /// Pending key-read responses.
    pub keys: RefCell<VecDeque<Key>>,
}

impl TerminalRecorder {
    /// Attaches a recorder to `channel` as the program page terminal and returns the shared recorder handle.
    pub fn serve<D: Driver>(channel: &Channel<Page, D>) -> Rc<Self> {
        let terminal = Rc::new(Self::default());

        channel.on::<OutputBatch>({
            let terminal = Rc::clone(&terminal);

            move |batch, _next| {
                let entries = batch.into_owned().entries.into_owned();

                for entry in &entries {
                    if let Output::Print { text, .. } = entry {
                        terminal.print_log.borrow_mut().push(text.clone());
                    }
                }

                terminal.batches.borrow_mut().push(entries);
            }
        });
        channel.handle({
            let terminal = Rc::clone(&terminal);

            move |_: ReadLine<'static>, response, _next| match terminal
                .lines
                .borrow_mut()
                .pop_front()
            {
                Some(line) => response.respond(line),
                None => response.fail(ReadFailure::Cancellation),
            }
        });
        channel.handle({
            let terminal = Rc::clone(&terminal);

            move |_: ReadKey<'static>, response, _next| match terminal.keys.borrow_mut().pop_front()
            {
                Some(key) => response.respond(key),
                None => response.fail(ReadFailure::Cancellation),
            }
        });

        terminal
    }
}
