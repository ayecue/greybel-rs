use greybel_documents::{DependencyResolver, ResourceRequest, Uri};

use crate::message::RequestId;

pub(in crate::server) struct PendingFetch {
    pub entry: Uri,
    pub resolver: DependencyResolver,
    pub batch: Vec<ResourceRequest>,
}

#[derive(Default)]
pub(in crate::server) struct FetchHandle {
    pub active: Vec<PendingFetch>,
    /// `custom/imported-files` requests responded to once their document's fetch completes.
    pub imported_file_queries: Vec<(RequestId, Uri)>,
}

impl FetchHandle {
    pub(in crate::server) fn is_loading(&self, uri: &Uri) -> bool {
        self.active.iter().any(|fetch| fetch.entry == *uri)
    }

    pub(in crate::server) fn cancel(&mut self, uri: &Uri) {
        self.active.retain(|fetch| fetch.entry != *uri);
    }
}
