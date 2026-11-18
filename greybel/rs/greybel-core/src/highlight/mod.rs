//! Token classification for syntax highlighting, shared by the language server's semantic tokens and the API
//! reference's examples.

mod kind;
mod roles;
mod scanner;

pub use self::kind::HighlightKind;
use self::roles::IdentifierRoles;
use self::scanner::TokenScanner;
use crate::{Ast, Dialect, Lexer, Span, TokenStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightToken {
    pub span: Span,
    pub kind: HighlightKind,
    /// Whether this token names an API function, as determined by `Highlighter::with_library`.
    pub library: bool,
}

/// Classifies an AST's tokens by its nodes: calls, parameters, properties, contextual keywords and types written in
/// code.
pub struct Highlighter<'a> {
    ast: &'a Ast,
    roles: IdentifierRoles<'a>,
    library: Option<&'a dyn Fn(&str) -> bool>,
}

impl<'a> Highlighter<'a> {
    #[must_use]
    pub fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            roles: IdentifierRoles::of(ast),
            library: None,
        }
    }

    /// Sets `HighlightToken::library` for identifiers accepted by `is_library`.
    #[must_use]
    pub fn with_library(self, is_library: &'a dyn Fn(&str) -> bool) -> Self {
        Self {
            library: Some(is_library),
            ..self
        }
    }

    /// In source order, without whitespace and line ends; stops at the first lexer error.
    #[must_use]
    pub fn tokens(&self, dialect: Dialect) -> Vec<HighlightToken> {
        let lexer = Lexer::new(self.ast.source(), dialect, dialect.default_version());
        let mut scanner = TokenScanner::new(&self.roles, self.library);
        let mut stream = TokenStream::new_unsafe();

        scanner.scan(&lexer, &mut stream);

        scanner.into_tokens()
    }
}
