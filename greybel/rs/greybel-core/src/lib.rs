#![warn(clippy::must_use_candidate)]

extern crate self as greybel_core;

pub mod ast;
pub mod diagnostics;
pub mod dialect;
pub mod error;
pub mod highlight;
pub mod lexer;
pub mod parser;
pub mod position;
pub mod syntax;

pub use ast::{
    Ast, AstNode, Atom, AtomTable, BinaryOp, BitwiseOp, Comment, DescendantIter, DumpContext,
    IdentifierKind, IfClauseKind, LiteralValue, LogicalOp, Node, NodeCommentContainer,
    NodeFieldContainer, NodeId, NodeKind, NodeStore, OperatorChain, ScopeData, TextSpan, UnaryOp,
    Visitor, span_location,
};
pub use dialect::{Dialect, Grammar, SyntaxFeatures, Version};
pub use error::{ErrorCode, ErrorKind, ParseError, ParseResult};
pub use lexer::{Keyword, Lexer, Punct, Token, TokenIter, TokenKind, TokenStream};
pub use parser::{
    ChunkData, ExprContext, InfixRule, KeywordStatement, MAX_NODE_DEPTH, ParseOutput, Parser,
    ParserOptions, PendingBlock, PrefixRule,
};
pub use position::{ColumnSettings, LineIndex, Position, PositionEncoding, Range, Span};

pub fn parse(source: &str, options: ParserOptions) -> Result<Ast, ParseError> {
    Parser::new(source, options).parse()
}

#[must_use]
pub fn parse_unsafe(source: &str, options: ParserOptions) -> ParseOutput {
    Parser::new(source, options).parse_unsafe()
}
