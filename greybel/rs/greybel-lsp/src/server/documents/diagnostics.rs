use gen_lsp_types as lsp;
use greybel_documents::Uri;

use crate::server::Server;
use crate::to_lsp::ToLsp;

impl Server {
    pub(in crate::server) fn publish_diagnostics(&mut self) {
        let options = self.settings.diagnostic_options();

        for uri in self.documents.uris() {
            if self.fetch_handle.is_loading(&uri) {
                continue;
            }

            let Some(document) = self.documents.get(&uri) else {
                continue;
            };
            let diagnostics: Vec<lsp::Diagnostic> = if self.settings.diagnostic {
                options
                    .apply(self.analyzer().all_diagnostics(&uri))
                    .iter()
                    .map(ToLsp::to_lsp)
                    .collect()
            } else {
                Vec::new()
            };

            if self.published_diagnostics.get(&uri) == Some(&diagnostics) {
                continue;
            }

            let params = lsp::PublishDiagnosticsParams {
                uri: document.uri.clone(),
                version: Some(document.version),
                diagnostics: diagnostics.clone(),
            };

            self.published_diagnostics.insert(uri, diagnostics);
            self.outbound
                .notify::<lsp::PublishDiagnosticsNotification>(params);
        }
    }

    pub(in crate::server) fn clear_diagnostics(&mut self, document: &Uri, uri: lsp::Uri) {
        if self
            .published_diagnostics
            .remove(document)
            .is_some_and(|diagnostics| !diagnostics.is_empty())
        {
            self.outbound.notify::<lsp::PublishDiagnosticsNotification>(
                lsp::PublishDiagnosticsParams {
                    uri,
                    version: None,
                    diagnostics: Vec::new(),
                },
            );
        }
    }
}
