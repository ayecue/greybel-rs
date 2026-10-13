use gen_lsp_types as lsp;

use super::Server;
use crate::error::Result;
use crate::to_lsp;

impl Server {
    pub(in crate::server) fn document_symbol(
        &mut self,
        params: lsp::DocumentSymbolParams,
    ) -> Result<Option<lsp::DocumentSymbolResponse>> {
        let Some(document) = self.document_analysis(&params.text_document.uri)? else {
            return Ok(None);
        };

        Ok(Some(lsp::DocumentSymbolResponse::DocumentSymbolList(
            document
                .analyzer
                .document_symbols(&document.uri)
                .iter()
                .map(to_lsp::document_symbol)
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
                .map(to_lsp::workspace_symbol)
                .collect(),
        )))
    }
}
