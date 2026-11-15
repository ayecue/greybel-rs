//! Generates LSP semantic tokens from lexer tokens and parser-resolved identifier roles with the
//! `greybel-languageserver` legend.

use std::collections::HashMap;

use gen_lsp_types as lsp;
use greybel_core::{
    Ast, ColumnSettings, Dialect, IdentifierKind, Lexer, Node, NodeId, NodeKind, TokenKind, Visitor,
};

/// Semantic token category represented by an index into [`TOKEN_TYPES`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum TokenType {
    Keyword,
    String,
    Number,
    Variable,
    Property,
    Function,
    Parameter,
    Operator,
    Comment,
    Constant,
    Punctuator,
}

/// LSP token type names ordered by [`TokenType`] discriminant.
const TOKEN_TYPES: [&str; 11] = [
    "keyword",
    "string",
    "number",
    "variable",
    "property",
    "function",
    "parameter",
    "operator",
    "comment",
    "constant",
    "punctuator",
];

/// Returns the server token legend without modifiers.
pub(crate) fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: TOKEN_TYPES.map(str::to_owned).to_vec(),
        token_modifiers: Vec::new(),
    }
}

/// Lexes `ast`'s source in `dialect` and returns semantic tokens, splitting multiline tokens by line.
pub(crate) fn semantic_tokens(ast: &Ast, dialect: Dialect) -> Vec<lsp::SemanticToken> {
    let identifiers = IdentifierTypeTable::of(ast);
    let source = ast.source();
    let lexer = Lexer::new(source, dialect, dialect.default_version());
    let mut encoder = TokenEncoder::new(ast);

    for token in lexer.tokens_unsafe() {
        let Ok(token) = token else { continue };
        let span = token.span();
        let token_type = match token.kind() {
            TokenKind::Keyword => TokenType::Keyword,
            TokenKind::StringLiteral => TokenType::String,
            TokenKind::NumericLiteral => TokenType::Number,
            TokenKind::BooleanLiteral | TokenKind::NilLiteral => TokenType::Constant,
            TokenKind::Comment => TokenType::Comment,
            TokenKind::Identifier => identifiers.of_token(span.start),
            TokenKind::Punctuator | TokenKind::SliceOperator => {
                if is_operator(&source[span.start as usize..span.end as usize]) {
                    TokenType::Operator
                } else {
                    TokenType::Punctuator
                }
            }
            TokenKind::Eof | TokenKind::Eol | TokenKind::Invalid => continue,
        };

        encoder.push(span.start, span.end, token_type);
    }

    encoder.tokens
}

/// Checks whether `text` is an operator, excluding brackets, separators, and member-access dots.
fn is_operator(text: &str) -> bool {
    !matches!(
        text,
        "(" | ")" | "[" | "]" | "{" | "}" | "," | "." | ":" | ";"
    )
}

/// Associates identifier start offsets with parser-resolved parameter, property, function, or variable token types.
struct IdentifierTypeTable<'a> {
    ast: &'a Ast,
    types: HashMap<u32, TokenType>,
}

impl<'a> IdentifierTypeTable<'a> {
    fn of(ast: &'a Ast) -> Self {
        let mut identifiers = Self {
            ast,
            types: HashMap::new(),
        };

        ast.walk(ast.root(), &mut identifiers);

        identifiers
    }

    fn of_token(&self, start: u32) -> TokenType {
        self.types
            .get(&start)
            .copied()
            .unwrap_or(TokenType::Variable)
    }

    /// Extracts the invoked identifier from `callee`, using either the callee or its member identifier; other
    /// expressions return `None`.
    fn called_identifier(&self, callee: NodeId) -> Option<NodeId> {
        match self.ast.kind(callee) {
            NodeKind::Identifier(_) => Some(callee),
            NodeKind::Member(member) => Some(member.identifier),
            _ => None,
        }
    }
}

impl Visitor for IdentifierTypeTable<'_> {
    fn enter(&mut self, _id: NodeId, node: &Node) -> bool {
        match &node.kind {
            NodeKind::Identifier(identifier) => {
                let token_type = match identifier.kind {
                    IdentifierKind::Argument => TokenType::Parameter,
                    IdentifierKind::Property => TokenType::Property,
                    IdentifierKind::Variable | IdentifierKind::ForInVariable => TokenType::Variable,
                };

                self.types.entry(node.span.start).or_insert(token_type);
            }
            NodeKind::Call(call) => {
                if let Some(callee) = self.called_identifier(call.base) {
                    self.types
                        .insert(self.ast.span(callee).start, TokenType::Function);
                }
            }
            NodeKind::CallStatement(statement) => {
                if let Some(callee) = self.called_identifier(statement.expression) {
                    self.types
                        .insert(self.ast.span(callee).start, TokenType::Function);
                }
            }
            _ => {}
        }

        true
    }
}

/// Encodes semantic tokens as LSP deltas from each previous token.
struct TokenEncoder<'a> {
    ast: &'a Ast,
    tokens: Vec<lsp::SemanticToken>,
    line: u32,
    character: u32,
}

impl<'a> TokenEncoder<'a> {
    fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            tokens: Vec::new(),
            line: 0,
            character: 0,
        }
    }

    /// Appends the token spanning `start..end`, split into tokens for nonempty line segments.
    fn push(&mut self, start: u32, end: u32, token_type: TokenType) {
        let text = &self.ast.source()[start as usize..end as usize];
        let mut offset = start;

        for line in text.split('\n') {
            let segment = line.strip_suffix('\r').unwrap_or(line);

            if !segment.is_empty() {
                self.push_segment(
                    offset,
                    offset
                        + u32::try_from(segment.len()).expect("value fits the target integer type"),
                    token_type,
                );
            }

            offset += u32::try_from(line.len()).expect("value fits the target integer type") + 1;
        }
    }

    fn push_segment(&mut self, start: u32, end: u32, token_type: TokenType) {
        let lines = self.ast.lines();
        let from = lines.position(start, ColumnSettings::default());
        let to = lines.position(end, ColumnSettings::default());
        let line = from.line - 1;
        let character = from.character - 1;
        let delta_line = line - self.line;
        let delta_start = if delta_line == 0 {
            character - self.character
        } else {
            character
        };

        self.tokens.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length: to.character - from.character,
            token_type: token_type as u32,
            token_modifiers_bitset: 0,
        });
        self.line = line;
        self.character = character;
    }
}
