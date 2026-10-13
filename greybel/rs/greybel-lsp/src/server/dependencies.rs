#[cfg(not(target_family = "wasm"))]
use greybel_documents::ResourceProvider;
use greybel_documents::{DependencyResolver, Resolution, ResourceRequest, ResourceResponse, Uri};
#[cfg(not(target_family = "wasm"))]
use greybel_documents_fs_provider::FsProvider;

use super::Server;
use super::state::{FileAccess, PendingFetch};

impl Server {
    pub(super) fn load(&mut self, uri: &Uri) {
        self.fetch_handle.cancel(uri);
        self.fetch_handle.active.push(PendingFetch {
            entry: uri.clone(),
            resolver: DependencyResolver::new(uri.clone()),
            batch: Vec::new(),
        });
        self.execute_all();
    }

    pub(super) fn reload_open_documents(&mut self) {
        for uri in self.documents.uris() {
            self.load(&uri);
        }
    }

    pub(super) fn execute_all(&mut self) {
        let active_fetches = std::mem::take(&mut self.fetch_handle.active);

        for mut fetch in active_fetches {
            if !self.finish_fetch(&mut fetch) {
                self.fetch_handle.active.push(fetch);
            }
        }
    }

    fn finish_fetch(&mut self, fetch: &mut PendingFetch) -> bool {
        loop {
            if fetch.batch.is_empty() {
                match fetch.resolver.next(&self.workspace) {
                    Resolution::Complete => return true,
                    Resolution::Requests(requests) => fetch.batch = requests,
                }
            }

            let responses: Vec<ResourceResponse> = fetch
                .batch
                .iter()
                .filter_map(|request| self.resource(request))
                .collect();

            if responses.len() < fetch.batch.len() {
                return false;
            }

            fetch
                .resolver
                .process_responses(&mut self.workspace, responses);
            fetch.batch.clear();
        }
    }

    fn resource(&mut self, request: &ResourceRequest) -> Option<ResourceResponse> {
        match request {
            ResourceRequest::RelativeTo { source, target } => {
                Some(ResourceResponse::Uri(self.relative_to(source, target)))
            }
            ResourceRequest::Read(uri) => {
                if self.workspace.file(uri).is_some() {
                    return Some(ResourceResponse::Content(
                        self.workspace.text(uri).map(|text| text.to_string()),
                    ));
                }

                match &self.access {
                    FileAccess::Client(_) => {
                        self.files.read(uri, &mut self.outbound);

                        None
                    }
                    #[cfg(not(target_family = "wasm"))]
                    FileAccess::Local(provider) => {
                        Some(ResourceResponse::Content(provider.read(uri).ok().flatten()))
                    }
                }
            }
        }
    }

    fn relative_to(&self, source: &Uri, target: &str) -> Uri {
        self.access
            .resolution()
            .relative_to(source, target, |uri| self.exists(uri))
    }

    fn exists(&self, uri: &Uri) -> bool {
        if self.files.contains(uri) || self.workspace.text(uri).is_some() {
            return true;
        }

        match &self.access {
            FileAccess::Client(_) => false,
            #[cfg(not(target_family = "wasm"))]
            FileAccess::Local(_) => FsProvider::exists(uri),
        }
    }
}
