use gen_lsp_types as lsp;
use greybel_core::{ColumnSettings, LineIndex};

use super::Server;
use super::state::ReadyDocument;
use crate::from_lsp;

impl Server {
    pub(super) fn did_open(&mut self, params: lsp::DidOpenTextDocumentParams) {
        let document = params.text_document;
        let Some(uri) = from_lsp::uri(&document.uri) else {
            return;
        };

        self.workspace.set_file(&uri, document.text);
        self.documents.open(
            uri.clone(),
            ReadyDocument {
                uri: document.uri,
                version: document.version,
            },
        );
        self.load(&uri);
    }

    pub(super) fn did_change(&mut self, params: lsp::DidChangeTextDocumentParams) {
        let Some(uri) = from_lsp::uri(&params.text_document.text_document_identifier.uri) else {
            return;
        };
        let Some(document) = self.documents.get_mut(&uri) else {
            return;
        };

        document.version = params.text_document.version;

        let mut text = self
            .workspace
            .text(&uri)
            .as_deref()
            .unwrap_or_default()
            .to_owned();

        for change in params.content_changes {
            Self::apply(&mut text, change);
        }

        self.workspace.set_file(&uri, text);
        self.load(&uri);
    }

    pub(super) fn did_close(&mut self, params: lsp::DidCloseTextDocumentParams) {
        let Some(uri) = from_lsp::uri(&params.text_document.uri) else {
            return;
        };
        let Some(document) = self.documents.close(&uri) else {
            return;
        };

        self.fetch_handle.cancel(&uri);
        self.clear_diagnostics(&uri, document.uri);
        self.read_file(&uri);
    }

    fn apply(text: &mut String, change: lsp::TextDocumentContentChangeEvent) {
        match change {
            lsp::TextDocumentContentChangeEvent::TextDocumentContentChangeWholeDocument(
                document,
            ) => {
                *text = document.text;
            }
            lsp::TextDocumentContentChangeEvent::TextDocumentContentChangePartial(partial) => {
                let lines = LineIndex::new(text);
                let offset = |position: lsp::Position| {
                    lines
                        .offset(from_lsp::position(position), ColumnSettings::default())
                        .map_or(text.len(), |offset| offset as usize)
                };
                let start = offset(partial.range.start);
                let end = offset(partial.range.end).max(start);

                text.replace_range(start..end, &partial.text);
            }
        }
    }
}
