mod encoder;

use gen_lsp_types as lsp;
use greybel_core::highlight::{HighlightKind, Highlighter};
use greybel_core::{Ast, Dialect};

use self::encoder::TokenEncoder;

pub(crate) struct SemanticTokens;

impl SemanticTokens {
    /// The modifier of API function names, bit 0.
    const DEFAULT_LIBRARY: &str = "defaultLibrary";

    pub(crate) fn legend() -> lsp::SemanticTokensLegend {
        lsp::SemanticTokensLegend {
            token_types: HighlightKind::ALL
                .map(|kind| kind.name().to_owned())
                .to_vec(),
            token_modifiers: vec![Self::DEFAULT_LIBRARY.to_owned()],
        }
    }

    pub(crate) fn of(ast: &Ast, dialect: Dialect) -> Vec<lsp::SemanticToken> {
        let mut encoder = TokenEncoder::new(ast);

        let is_library = |name: &str| greyscript_meta::is_api_function(name);
        let highlighter = match dialect {
            Dialect::MiniScript => Highlighter::new(ast),
            Dialect::Greybel | Dialect::GreyScript => {
                Highlighter::new(ast).with_library(&is_library)
            }
        };

        for token in highlighter.tokens(dialect) {
            encoder.push(token);
        }

        encoder.into_tokens()
    }
}
