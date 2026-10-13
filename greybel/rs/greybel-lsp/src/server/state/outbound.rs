use gen_lsp_types::{Notification as LspNotification, Request as LspRequest};
use greybel_documents::Uri;
use rustc_hash::FxHashMap;

use crate::message::{Message, Notification, Request, RequestId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::server) enum Expectation {
    Configuration,
    FindFiles,
    Read(Uri),
    ProjectFile(Uri),
}

#[derive(Debug, Default)]
pub(in crate::server) struct Outbound {
    messages: Vec<Message>,
    next_id: i64,
    awaited_responses: FxHashMap<RequestId, Expectation>,
}

impl Outbound {
    pub(in crate::server) fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub(in crate::server) fn request<R: LspRequest>(
        &mut self,
        params: R::Params,
        expectation: Expectation,
    ) {
        let id = RequestId::Number(self.next_id);

        self.next_id += 1;
        self.awaited_responses.insert(id.clone(), expectation);
        self.messages
            .push(Message::Request(Request::new::<R>(id, params)));
    }

    pub(in crate::server) fn notify<N: LspNotification>(&mut self, params: N::Params) {
        self.messages
            .push(Message::Notification(Notification::new::<N>(params)));
    }

    pub(in crate::server) fn resolve(&mut self, id: &RequestId) -> Option<Expectation> {
        self.awaited_responses.remove(id)
    }

    pub(in crate::server) fn take(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.messages)
    }
}
