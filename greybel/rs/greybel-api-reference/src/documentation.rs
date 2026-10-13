//! Documentation request handling and a service loop that streams references sequentially.

use std::pin::pin;

use duplex_core::{
    Channel, ChannelOptions, Driver, NextMessage, Responder, StreamResponder, Transport,
};
use futures_util::future::{Either, select};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::protocol::{
    Block, Documentation as DocumentationPeer, DocumentationRouter, Overview, Reference, Render,
    Search, SearchResult, Section,
};
use crate::section::SectionWriter;

/// A queued reference request whose response sections follow the reader's stream credit.
type ReferenceRequest = (Reference, StreamResponder<Overview, (), Section>);

/// Serves one duplex reader. References stream sequentially, while the request handler answers searches and renders
/// immediately.
pub struct Documentation<D: Driver> {
    channel: Channel<DocumentationPeer, D>,
    requests: UnboundedReceiver<ReferenceRequest>,
}

impl<D: Driver> Documentation<D> {
    /// Creates an API reference provider on `transport` and registers its request handlers.
    #[must_use]
    pub fn new(transport: &Transport<D>) -> Self {
        let (sender, requests) = mpsc::unbounded_channel();
        let channel = Channel::new(transport, ChannelOptions::default());

        DocumentationHandler(sender).attach(&channel);

        Self { channel, requests }
    }

    /// Streams references until the reader closes the protocol or the request channel closes.
    pub async fn serve(mut self) {
        let mut close_event = pin!(self.channel.close_event());

        loop {
            let (_, responder) = {
                let next = pin!(self.requests.recv());

                match select(next, close_event.as_mut()).await {
                    Either::Left((Some(request), _)) => request,
                    Either::Left((None, _)) | Either::Right(_) => return,
                }
            };
            let writer = SectionWriter::new();
            let mut items = responder.start(Overview::new());

            if items.send_all(writer.sections()).await.is_ok() {
                items.finish();
            }
        }
    }
}

/// Enqueues reference requests in arrival order and responds to searches and renders synchronously.
struct DocumentationHandler(UnboundedSender<ReferenceRequest>);

impl DocumentationRouter for DocumentationHandler {
    fn on_reference(
        &self,
        message: Reference,
        responder: StreamResponder<Overview, (), Section>,
        _next: NextMessage<Reference>,
    ) {
        let _ = self.0.send((message, responder));
    }

    fn on_search(
        &self,
        message: Search<'static>,
        responder: Responder<SearchResult>,
        _next: NextMessage<Search<'static>>,
    ) {
        responder.respond(SearchResult::of(&message));
    }

    fn on_render(
        &self,
        message: Render<'static>,
        responder: Responder<Vec<Block>>,
        _next: NextMessage<Render<'static>>,
    ) {
        responder.respond(SectionWriter::new().blocks(&message.markdown));
    }
}
