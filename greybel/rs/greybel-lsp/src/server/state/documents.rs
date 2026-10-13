use gen_lsp_types as lsp;
use greybel_documents::Uri;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::server) struct ReadyDocument {
    pub uri: lsp::Uri,
    pub version: i32,
}

#[derive(Debug, Default)]
pub(in crate::server) struct DocumentTable {
    open: FxHashMap<Uri, ReadyDocument>,
}

impl DocumentTable {
    pub(in crate::server) fn open(&mut self, uri: Uri, document: ReadyDocument) {
        self.open.insert(uri, document);
    }

    pub(in crate::server) fn close(&mut self, uri: &Uri) -> Option<ReadyDocument> {
        self.open.remove(uri)
    }

    pub(in crate::server) fn get(&self, uri: &Uri) -> Option<&ReadyDocument> {
        self.open.get(uri)
    }

    pub(in crate::server) fn get_mut(&mut self, uri: &Uri) -> Option<&mut ReadyDocument> {
        self.open.get_mut(uri)
    }

    pub(in crate::server) fn is_open(&self, uri: &Uri) -> bool {
        self.open.contains_key(uri)
    }

    pub(in crate::server) fn uris(&self) -> Vec<Uri> {
        let mut uris: Vec<Uri> = self.open.keys().cloned().collect();

        uris.sort();

        uris
    }
}
