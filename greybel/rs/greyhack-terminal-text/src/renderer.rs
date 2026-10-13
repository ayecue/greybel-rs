use duplex_core::{Channel, ChannelOptions, Driver, NextMessage, Responder, Transport};

use crate::protocol::{AnsiOutput, Render, Renderer as RendererRole, RendererRouter, RichText};

/// Duplex terminal-text provider that renders requests for one terminal.
pub struct Renderer<D: Driver> {
    channel: Channel<RendererRole, D>,
}

impl<D: Driver> Renderer<D> {
    /// Creates a terminal-text provider on `transport` and registers its render handler.
    #[must_use]
    pub fn new(transport: &Transport<D>) -> Self {
        let channel = Channel::new(transport, ChannelOptions::default());

        RenderHandler.attach(&channel);

        Self { channel }
    }

    /// Handles render requests until the terminal closes the protocol.
    pub async fn serve(self) {
        self.channel.close_event().await;
    }
}

/// Synchronously renders every text in each `Render` request.
struct RenderHandler;

impl RendererRouter for RenderHandler {
    fn on_render(
        &self,
        message: Render<'static>,
        responder: Responder<AnsiOutput>,
        _next: NextMessage<Render<'static>>,
    ) {
        responder.respond(AnsiOutput {
            texts: message
                .texts
                .iter()
                .map(|rich_text: &RichText| crate::ansi_text(&rich_text.text, rich_text.columns))
                .collect(),
        });
    }
}
