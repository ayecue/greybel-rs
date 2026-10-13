//! Decodes LSP semantic tokens for assertions.

use greybel_lsp::types as lsp;

/// Converts delta-encoded tokens to absolute `(line, character, length, token_type)` tuples using `legend`.
pub fn absolute(tokens: &[lsp::SemanticToken], legend: &[String]) -> Vec<(u32, u32, u32, String)> {
    let mut line = 0;
    let mut character = 0;

    tokens
        .iter()
        .map(|token| {
            if token.delta_line > 0 {
                line += token.delta_line;
                character = token.delta_start;
            } else {
                character += token.delta_start;
            }

            (
                line,
                character,
                token.length,
                legend[token.token_type as usize].clone(),
            )
        })
        .collect()
}
