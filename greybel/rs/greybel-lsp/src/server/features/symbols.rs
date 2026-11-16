use gen_lsp_types as lsp;

use super::{AnalyzedDocument, Server};
use crate::error::Result;
use crate::to_lsp::ToLsp;

impl Server {
    pub(in crate::server) fn document_symbol(
        &mut self,
        params: lsp::DocumentSymbolParams,
    ) -> Result<Option<lsp::DocumentSymbolResponse>> {
        let Some(document) = AnalyzedDocument::of(self, &params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(lsp::DocumentSymbolResponse::DocumentSymbolList(
            document
                .analyzer
                .document_symbols(&document.uri)
                .iter()
                .map(ToLsp::to_lsp)
                .collect(),
        )))
    }

    pub(in crate::server) fn workspace_symbol(
        &mut self,
        params: lsp::WorkspaceSymbolParams,
    ) -> Result<Option<lsp::WorkspaceSymbolResponse>> {
        Ok(Some(lsp::WorkspaceSymbolResponse::WorkspaceSymbolList(
            self.analyzer()
                .workspace_symbols(&params.query)
                .iter()
                .map(ToLsp::to_lsp)
                .collect(),
        )))
    }
}
