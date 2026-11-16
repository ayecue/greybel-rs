use greybel_documents::Uri;
use rustc_hash::FxHashSet;

use crate::client_requests::{ReadFile, ReadFileRequest};
use crate::server::state::{Expectation, Outbound};

/// Tracks URIs found by client searches and URIs already requested for reading.
#[derive(Debug, Default)]
pub(in crate::server) struct ClientFileTable {
    uris: FxHashSet<Uri>,
    reads: FxHashSet<Uri>,
}

impl ClientFileTable {
    pub(in crate::server) fn contains(&self, uri: &Uri) -> bool {
        self.uris.contains(uri)
    }

    pub(in crate::server) fn uris(&self) -> Vec<Uri> {
        let mut uris: Vec<Uri> = self.uris.iter().cloned().collect();

        uris.sort();

        uris
    }

    pub(in crate::server) fn replace_uris(&mut self, uris: FxHashSet<Uri>) {
        self.uris = uris;
    }

    pub(in crate::server) fn remove(&mut self, uri: &Uri) {
        self.uris.remove(uri);
    }

    pub(in crate::server) fn read(&mut self, uri: &Uri, outbound: &mut Outbound) {
        if !self.reads.insert(uri.clone()) {
            return;
        }

        let params = ReadFileRequest {
            uri: uri.to_string(),
        };

        outbound.request::<ReadFile>(params, Expectation::Read(uri.clone()));
    }

    pub(in crate::server) fn finish_read(&mut self, uri: &Uri) {
        self.reads.remove(uri);
    }
}
