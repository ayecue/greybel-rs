mod encoder;
mod roles;
mod scanner;
mod token_type;

use gen_lsp_types as lsp;
use greybel_core::{Ast, Dialect, Lexer, TokenStream};

use self::encoder::TokenEncoder;
use self::roles::IdentifierRoles;
use self::scanner::TokenScanner;
use self::token_type::TokenType;

pub(crate) struct SemanticTokens;

impl SemanticTokens {
    pub(crate) fn legend() -> lsp::SemanticTokensLegend {
        lsp::SemanticTokensLegend {
            token_types: TokenType::ALL
                .map(|token_type| token_type.name().to_owned())
                .to_vec(),
            token_modifiers: Vec::new(),
        }
    }

    pub(crate) fn of(ast: &Ast, dialect: Dialect) -> Vec<lsp::SemanticToken> {
        let roles = IdentifierRoles::of(ast);
        let lexer = Lexer::new(ast.source(), dialect, dialect.default_version());
        let mut scanner = TokenScanner::new(&roles, TokenEncoder::new(ast));
        let mut stream = TokenStream::new_unsafe();

        scanner.scan(&lexer, &mut stream);

        scanner.into_tokens()
    }
}
