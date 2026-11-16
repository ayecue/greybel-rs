//! Generates LSP semantic tokens from lexer tokens and parser-resolved identifier roles with the
//! `greybel-languageserver` legend.

use std::collections::HashMap;

use gen_lsp_types as lsp;
use greybel_core::lexer::TemplateScan;
use greybel_core::syntax::greybel::TypeBody;
use greybel_core::{
    Ast, ColumnSettings, Dialect, IdentifierKind, Lexer, Node, NodeId, NodeKind, Span, TokenKind,
    TokenStream, Visitor,
};
use greybel_symbols::contextual_keywords;

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
    Type,
}

/// LSP token type names ordered by [`TokenType`] discriminant.
const TOKEN_TYPES: [&str; 12] = [
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
    "type",
];

/// Words that are keywords where no identifier of the syntax tree is: `let`, `as`, `class`, `case is`, ...
const CONTEXTUAL_KEYWORDS: [&str; 17] = [
    contextual_keywords::FROM,
    contextual_keywords::LET,
    contextual_keywords::CONST,
    contextual_keywords::ENUM,
    contextual_keywords::ASSERT,
    contextual_keywords::AS,
    contextual_keywords::SWITCH,
    contextual_keywords::CASE,
    contextual_keywords::IS,
    contextual_keywords::EXPORT,
    contextual_keywords::CLASS,
    contextual_keywords::EXTENDS,
    contextual_keywords::IMPLEMENTS,
    contextual_keywords::PRIVATE,
    contextual_keywords::STATIC,
    contextual_keywords::READONLY,
    contextual_keywords::TYPE,
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
    let mut stream = TokenStream::new_unsafe();

    encoder.push_tokens(&identifiers, &lexer, &mut stream);

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
    /// The spans of the types written in code, sorted by start.
    type_spans: Vec<Span>,
    ast: &'a Ast,
    types: HashMap<u32, TokenType>,
}

impl<'a> IdentifierTypeTable<'a> {
    fn of(ast: &'a Ast) -> Self {
        let mut type_spans = ast.type_spans();

        type_spans.sort_unstable_by_key(|span| span.start);

        let mut identifiers = Self {
            type_spans,
            ast,
            types: HashMap::new(),
        };

        ast.walk(ast.root(), &mut identifiers);

        identifiers
    }

    /// The token type of the identifier token at `span`: its role in the syntax tree; otherwise a keyword for a
    /// contextual keyword and a type inside a type.
    fn of_token(&self, span: Span) -> TokenType {
        if let Some(&token_type) = self.types.get(&span.start) {
            return token_type;
        }

        let text = &self.ast.source()[span.start as usize..span.end as usize];

        if CONTEXTUAL_KEYWORDS.contains(&text) {
            return TokenType::Keyword;
        }

        let index = self.type_spans.partition_point(|ty| ty.start <= span.start);
        let in_type = self.type_spans[..index].iter().any(|ty| span.end <= ty.end);

        if in_type {
            TokenType::Type
        } else {
            TokenType::Variable
        }
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
            NodeKind::TypeStatement(statement) => {
                if let TypeBody::Members { members, .. } = &statement.body {
                    for member in members {
                        let token_type = match member.parameters {
                            Some(_) => TokenType::Function,
                            None => TokenType::Property,
                        };

                        self.types.insert(member.name.start, token_type);

                        for &(parameter, _) in member.parameters.iter().flatten() {
                            self.types.insert(parameter.start, TokenType::Parameter);
                        }
                    }
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
    /// Encodes the tokens `lexer` reads from the cursor of `stream` to the end of its source.
    fn push_tokens(
        &mut self,
        identifiers: &IdentifierTypeTable<'_>,
        lexer: &Lexer<'_>,
        stream: &mut TokenStream,
    ) {
        let source = lexer.source();

        while let Ok(token) = lexer.next_token(stream) {
            let span = token.span();
            let token_type = match token.kind() {
                TokenKind::Eof => break,
                _ if token.as_template().is_some() => {
                    self.push_template(identifiers, lexer, span);
                    continue;
                }
                TokenKind::Keyword => TokenType::Keyword,
                TokenKind::StringLiteral => TokenType::String,
                TokenKind::NumericLiteral => TokenType::Number,
                TokenKind::BooleanLiteral | TokenKind::NilLiteral => TokenType::Constant,
                TokenKind::Comment => TokenType::Comment,
                TokenKind::Identifier => identifiers.of_token(span),
                TokenKind::Punctuator | TokenKind::SliceOperator => {
                    if is_operator(&source[span.start as usize..span.end as usize]) {
                        TokenType::Operator
                    } else {
                        TokenType::Punctuator
                    }
                }
                TokenKind::Eol | TokenKind::Invalid => continue,
            };

            self.push(span.start, span.end, token_type);
        }
    }

    /// A `#"..."` template: its text as strings and the code in each `{...}` as tokens.
    fn push_template(
        &mut self,
        identifiers: &IdentifierTypeTable<'_>,
        lexer: &Lexer<'_>,
        span: Span,
    ) {
        let source = lexer.source();
        let Some(shape) = TemplateScan::new(source, true).template(span.start as usize) else {
            self.push(span.start, span.end, TokenType::String);

            return;
        };
        let mut text_start = span.start;

        for placeholder in shape.placeholders {
            self.push(text_start, placeholder.start, TokenType::String);

            let placeholder_lexer =
                Lexer::with_grammar(&source[..placeholder.end as usize], lexer.grammar());
            let mut stream = TokenStream::new_unsafe();

            stream.cursor.set(placeholder.start as usize);
            self.push_tokens(identifiers, &placeholder_lexer, &mut stream);
            text_start = placeholder.end;
        }

        self.push(text_start, span.end, TokenType::String);
    }

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
