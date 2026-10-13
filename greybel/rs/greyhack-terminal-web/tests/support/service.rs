//! In-memory terminal service with a drawing page and a printing program.

use std::borrow::Cow;
use std::cell::Cell;
use std::future::Future;
use std::rc::Rc;

use duplex_core::{Channel, ChannelOptions, NextNotification};
use duplex_test_support::{Memory, MemoryQueue};
use greybel_io::{Output, PrintOptions};
use greybel_io_web::protocol::{OutputBatch, Program};
use greyhack_terminal_web::TerminalService;
use greyhack_terminal_web::protocol::{io, terminal};
use page::{Draw, Page, PageRouter, Redraw, Scene, Viewport};

/// Client-side terminal protocol bindings used by tests acting as the page.
pub mod page {
    duplex_core::include_protocol!("$PROTOCOLS/terminal", major = 1, role = client);
}

/// Test viewport: 1000 × 300 CSS pixels at device-pixel ratio 2.
pub const VIEWPORT: Viewport = Viewport {
    width: 1000.0,
    height: 300.0,
    pixel_ratio: 2.0,
};

/// Memory harness with the terminal on one transport and its page and program on the other.
pub struct TerminalHarness {
    pub queue: MemoryQueue,
    pub page: Channel<Page, Memory>,
    pub program: Channel<Program, Memory>,
    /// Number of `Redraw` notifications received by the page.
    pub redraws: Rc<Cell<usize>>,
    _terminal: (Channel<terminal::View, Memory>, Channel<io::Page, Memory>),
}

impl TerminalHarness {
    pub fn new() -> Self {
        let (page_transport, terminal_transport, queue) = Memory::transports();
        let view = Channel::new(&terminal_transport, ChannelOptions::default());
        let io_page = Channel::new(&terminal_transport, ChannelOptions::default());

        TerminalService::serve(&view, &io_page);

        let page = Channel::new(&page_transport, ChannelOptions::default());
        let program = Channel::new(&page_transport, ChannelOptions::default());
        let redraws = Rc::default();

        RedrawCounter(Rc::clone(&redraws)).attach(&page);
        queue.deliver();

        Self {
            queue,
            page,
            program,
            redraws,
            _terminal: (view, io_page),
        }
    }

    /// Sends `text` as a newline-terminated program output batch and delivers it.
    pub fn print(&self, text: &str) {
        let entries = [Output::Print {
            text: text.to_owned(),
            options: PrintOptions {
                new_line: true,
                replace: false,
            },
        }];

        self.program
            .send(&OutputBatch {
                entries: Cow::Borrowed(&entries),
            })
            .expect("sent");
        self.queue.deliver();
    }

    /// Requests and returns the terminal scene for [`VIEWPORT`].
    pub fn draw(&self) -> Scene {
        let pending = self.page.send(&Draw { viewport: VIEWPORT }).expect("sent");

        self.wait(pending).expect("drawn")
    }

    /// Drives the memory queue until `future` is ready.
    pub fn wait<F: Future>(&self, future: F) -> F::Output {
        self.queue.drive(future)
    }
}

/// Counts `Redraw` notifications sent by the terminal.
struct RedrawCounter(Rc<Cell<usize>>);

impl PageRouter for RedrawCounter {
    fn on_redraw(&self, _message: Redraw, _next: NextNotification<Redraw>) {
        self.0.set(self.0.get() + 1);
    }
}
