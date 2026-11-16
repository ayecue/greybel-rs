use gen_lsp_types as lsp;

use super::Server;
use crate::error::Result;
use crate::semantic_tokens::SemanticTokens;
use crate::to_lsp;

impl Server {
    pub(in crate::server) fn semantic_tokens_full(
        &mut self,
        params: lsp::SemanticTokensParams,
    ) -> Result<Option<lsp::SemanticTokens>> {
        let uri = Self::requested_uri(&params.text_document.uri)?;
        let Some(file) = self.workspace.file(&uri) else {
            return Ok(None);
        };
        let parse = file.parse(self.workspace.db());
        let Some(ast) = parse.ast() else {
            return Ok(None);
        };

        Ok(Some(to_lsp::semantic_tokens(SemanticTokens::of(
            ast,
            self.dialect,
        ))))
    }
}
