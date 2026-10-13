use greybel_documents::{DependencyResolver, ResourceRequest, Uri};

pub(in crate::server) struct PendingFetch {
    pub entry: Uri,
    pub resolver: DependencyResolver,
    pub batch: Vec<ResourceRequest>,
}

#[derive(Default)]
pub(in crate::server) struct FetchHandle {
    pub active: Vec<PendingFetch>,
}

impl FetchHandle {
    pub(in crate::server) fn is_loading(&self, uri: &Uri) -> bool {
        self.active.iter().any(|fetch| fetch.entry == *uri)
    }

    pub(in crate::server) fn cancel(&mut self, uri: &Uri) {
        self.active.retain(|fetch| fetch.entry != *uri);
    }
}
