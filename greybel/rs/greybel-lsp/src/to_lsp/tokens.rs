use gen_lsp_types as lsp;

pub(crate) fn semantic_tokens(data: Vec<lsp::SemanticToken>) -> lsp::SemanticTokens {
    lsp::SemanticTokens {
        result_id: None,
        data,
    }
}
