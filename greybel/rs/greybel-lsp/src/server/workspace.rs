use gen_lsp_types as lsp;
#[cfg(not(target_family = "wasm"))]
use greybel_documents::ResourceProvider;
use greybel_documents::Uri;
use rustc_hash::FxHashSet;

use super::Server;
use super::state::{Expectation, FileAccess};
use crate::files::{FileSearch, FindFilesRequest};
use crate::from_lsp;

impl Server {
    pub(super) fn find_files(&mut self) {
        let exclude = &self.settings.type_analyzer.exclude;
        let params = FindFilesRequest {
            include: self.settings.file_pattern(),
            exclude: (!exclude.is_empty()).then(|| exclude.clone()),
        };

        match &self.access {
            FileAccess::Client(_) => self
                .outbound
                .request::<FileSearch>(params, Expectation::FileSearch),
            #[cfg(not(target_family = "wasm"))]
            FileAccess::Local(provider) => {
                let uris = provider.find(&params.include, params.exclude.as_deref());

                self.handle_find_files(uris.into_iter().collect());
            }
        }
    }

    pub(super) fn handle_find_files(&mut self, uris: FxHashSet<Uri>) {
        self.files.replace_uris(uris);

        for uri in self.files.uris() {
            if !self.documents.is_open(&uri) && self.workspace.text(&uri).is_none() {
                self.read_file(&uri);
            }
        }

        self.reload_open_documents();
    }

    pub(super) fn read_file(&mut self, uri: &Uri) {
        match &self.access {
            FileAccess::Client(_) => self.files.read(uri, &mut self.outbound),
            #[cfg(not(target_family = "wasm"))]
            FileAccess::Local(provider) => {
                let content = provider.read(uri).ok().flatten();

                self.handle_file_read(uri, content);
            }
        }
    }

    pub(super) fn handle_file_read(&mut self, uri: &Uri, content: Option<String>) {
        self.files.finish_read(uri);

        if !self.documents.is_open(uri) {
            match content {
                Some(text) if self.workspace.text(uri).as_deref() != Some(text.as_str()) => {
                    self.workspace.set_file(uri, text);
                    self.load(uri);
                }
                Some(_) => {}
                None => {
                    self.workspace.remove_file(uri);
                }
            }
        }

        self.execute_all();
    }

    pub(super) fn did_change_watched_files(&mut self, params: lsp::DidChangeWatchedFilesParams) {
        let mut created = false;

        for change in params.changes {
            let Some(uri) = from_lsp::uri(&change.uri) else {
                continue;
            };
            let is_stored = !self.documents.is_open(&uri) && self.workspace.file(&uri).is_some();

            match change.kind {
                lsp::FileChangeType::Created | lsp::FileChangeType::Changed => {
                    created |= change.kind == lsp::FileChangeType::Created;
                    if is_stored {
                        self.read_file(&uri);
                    }
                }
                lsp::FileChangeType::Deleted => {
                    self.files.remove(&uri);
                    if is_stored {
                        self.workspace.remove_file(&uri);
                    }
                }
                lsp::FileChangeType::Custom(_) => {}
            }
        }

        if created {
            self.find_files();
        }

        self.reload_open_documents();
    }
}
