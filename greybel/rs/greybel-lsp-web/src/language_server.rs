use std::borrow::Cow;
use std::cell::RefCell;

use duplex_core::{Channel, Driver, ErrorCode, NextNotification};
use greybel_lsp::types::ErrorCodes;
use greybel_lsp::{Message, Response, ResponseError, Server as LspServer};

use crate::protocol::{ClientMessage, ClientMessageRef, Server, ServerMessage, ServerRouter};

/// Duplex language-server handler forwarding client JSON-RPC to `greybel-lsp`, returning replies, and closing the
/// channel when the client exits.
pub(crate) struct LanguageServer<D: Driver> {
    channel: Channel<Server, D>,
    server: RefCell<LspServer>,
}

impl<D: Driver> LanguageServer<D> {
    pub(crate) fn serve(channel: &Channel<Server, D>) {
        Self {
            channel: channel.clone(),
            server: RefCell::new(LspServer::new()),
        }
        .attach(channel);
    }

    /// Deserializes `json` into a client message and returns server replies; malformed JSON returns `InvalidRequest`.
    fn receive(&self, json: &str) -> Vec<Message> {
        match serde_json::from_str::<Message>(json) {
            Ok(message) => self.server.borrow_mut().receive(message),
            Err(error) => vec![Message::Response(Response::error(
                None,
                ResponseError::new(ErrorCodes::InvalidRequest, error.to_string()),
            ))],
        }
    }
}

impl<D: Driver> ServerRouter for LanguageServer<D> {
    fn on_client_message(
        &self,
        message: ClientMessageRef<'_>,
        _next: NextNotification<ClientMessage<'static>>,
    ) {
        for reply in self.receive(message.json) {
            let json = serde_json::to_string(&reply).expect("a message is plain JSON");
            let _ = self.channel.send(&ServerMessage {
                json: Cow::Owned(json),
            });
        }

        if self.server.borrow().exit_code().is_some() {
            self.channel.close(ErrorCode::ClosedChannel);
        }
    }
}
