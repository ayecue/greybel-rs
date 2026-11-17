mod encoder;

use gen_lsp_types as lsp;
use greybel_core::highlight::{HighlightKind, Highlighter};
use greybel_core::{Ast, Dialect};

use self::encoder::TokenEncoder;

pub(crate) struct SemanticTokens;

impl SemanticTokens {
    pub(crate) fn legend() -> lsp::SemanticTokensLegend {
        lsp::SemanticTokensLegend {
            token_types: HighlightKind::ALL
                .map(|kind| kind.name().to_owned())
                .to_vec(),
            token_modifiers: Vec::new(),
        }
    }

    pub(crate) fn of(ast: &Ast, dialect: Dialect) -> Vec<lsp::SemanticToken> {
        let mut encoder = TokenEncoder::new(ast);

        for token in Highlighter::new(ast).tokens(dialect) {
            encoder.push(token);
        }

        encoder.into_tokens()
    }
}
