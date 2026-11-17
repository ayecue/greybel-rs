use std::cell::RefCell;
use std::rc::Rc;

use duplex_core::{Channel, Driver, NextMessage, NextNotification, Responder};
use greybel_io::OutputRef;
use text_mesh_core::LineSplitRule;

use crate::protocol::io::{self, OutputBatch, OutputBatchRef, PageRouter};
use crate::protocol::terminal::{
    Draw, Redraw, Scene, Scroll, SetLineLimits, TextureFailure, TextureSet, Textures, View,
    ViewRouter,
};
use crate::terminal::Terminal;

/// Displays program io output on the page and sends `Redraw` after each output batch. Program input is unsupported.
pub struct TerminalService<D: Driver> {
    terminal: RefCell<Terminal>,
    view: Channel<View, D>,
}

impl<D: Driver> TerminalService<D> {
    /// Registers terminal-protocol handlers on `view` and io output handlers on `page`.
    pub fn serve(view: &Channel<View, D>, page: &Channel<io::Page, D>) {
        let service = Rc::new(Self {
            terminal: RefCell::new(Terminal::new()),
            view: view.clone(),
        });

        ViewHandler(Rc::clone(&service)).attach(view);
        PageHandler(service).attach(page);
    }
}

/// Processes page texture, draw, and scroll requests.
struct ViewHandler<D: Driver>(Rc<TerminalService<D>>);

impl<D: Driver> ViewRouter for ViewHandler<D> {
    fn on_textures(
        &self,
        _message: Textures,
        responder: Responder<TextureSet, TextureFailure>,
        _next: NextMessage<Textures>,
    ) {
        match Terminal::textures() {
            Ok(textures) => responder.respond(textures),
            Err(failure) => responder.fail(failure),
        }
    }

    fn on_draw(&self, message: Draw, responder: Responder<Scene>, _next: NextMessage<Draw>) {
        responder.respond(self.0.terminal.borrow_mut().draw(&message.viewport));
    }

    fn on_scroll(&self, message: Scroll, _next: NextNotification<Scroll>) {
        self.0.terminal.borrow_mut().scroll_by(message.delta);
    }

    fn on_set_line_limits(&self, message: SetLineLimits, _next: NextNotification<SetLineLimits>) {
        let split_rule = message
            .limits
            .map_or(LineSplitRule::Game, |limits| limits.split_rule());

        self.0.terminal.borrow_mut().set_split_rule(split_rule);

        let _ = self.0.view.send(&Redraw {});
    }
}

/// Processes program output batches and triggers page redraws.
struct PageHandler<D: Driver>(Rc<TerminalService<D>>);

impl<D: Driver> PageRouter for PageHandler<D> {
    fn on_output_batch(&self, message: OutputBatchRef<'_>, _next: NextNotification<OutputBatch>) {
        let output: Vec<_> = message
            .entries
            .into_iter()
            .map(OutputRef::into_owned)
            .collect();

        self.0.terminal.borrow_mut().show(&output);

        let _ = self.0.view.send(&Redraw {});
    }
}
