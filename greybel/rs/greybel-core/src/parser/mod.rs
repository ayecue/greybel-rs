mod comments;
mod expect;
mod expression;
mod precedence;
pub mod rules;
pub mod state;
mod statement;
mod stream;
mod unsafe_mode;

use std::collections::HashMap;

pub use expect::{Expect, groups};
use greybel_symbols::names::UNNAMED_SOURCE;
pub use precedence::{Precedence, PrecedenceTable};
pub use rules::{
    Associativity, ExprContext, InfixRule, KeywordStatement, PrefixRule, RuleIndex, RuleTableSet,
    Trigger,
};
pub use state::{
    BlockStack, ChunkData, ChunkDataSnapshot, ChunkDataStack, CommentAttachment, ParseErrorStorage,
    PendingBlock, PendingScope, ScopeStack,
};
pub use stream::{ParseSnapshot, ParseStream};

use crate::ast::{Ast, NodeId, NodeKind};
use crate::diagnostics::{BlockDiagnostic, DepthDiagnostic, Diagnostic, ReportedLine};
use crate::dialect::{Dialect, Grammar, SyntaxFeatures, Version};
use crate::error::{ParseError, ParseResult};
use crate::lexer::{AnyToken, Lexer};
use crate::position::{LineIndex, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserOptions {
    pub dialect: Dialect,
    pub version: Version,
    pub filename: String,
    pub environment: HashMap<String, String>,
    pub preserve: bool,
    pub strict_mode: bool,
    pub mono_number_parsing: bool,
    pub syntax_features: SyntaxFeatures,
}

pub const MAX_NODE_DEPTH: usize = 256;

impl ParserOptions {
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        Self {
            dialect,
            version: dialect.default_version(),
            filename: UNNAMED_SOURCE.to_owned(),
            environment: HashMap::new(),
            preserve: false,
            strict_mode: false,
            mono_number_parsing: false,
            syntax_features: SyntaxFeatures::ALL,
        }
    }

    #[must_use]
    pub fn grammar(&self) -> &'static Grammar {
        Grammar::with_features(
            self.dialect,
            self.version,
            self.strict_mode,
            self.syntax_features,
        )
    }
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self::new(Dialect::MiniScript)
    }
}

pub struct ParseOutput {
    pub ast: Ast,
    pub errors: Vec<ParseError>,
    pub lexer_errors: Vec<ParseError>,
}

impl ParseOutput {
    pub fn all_errors(&self) -> impl Iterator<Item = &ParseError> {
        self.lexer_errors.iter().chain(&self.errors)
    }
}

pub struct Parser<'src> {
    pub(crate) lexer: Lexer<'src>,
    pub stream: ParseStream,
    grammar: &'static Grammar,
    options: ParserOptions,
}

impl<'src> Parser<'src> {
    #[must_use]
    pub fn new(source: &'src str, options: ParserOptions) -> Self {
        Self::from_lexer(&Lexer::with_grammar(source, options.grammar()), options)
    }

    #[must_use]
    pub fn from_lexer(lexer: &Lexer<'src>, options: ParserOptions) -> Self {
        Self {
            lexer: lexer.clone(),
            stream: ParseStream::new(),
            grammar: lexer.grammar(),
            options,
        }
    }

    pub fn parse(mut self) -> Result<Ast, ParseError> {
        if let Err(error) = self.parse_chunk() {
            let mut error = *error;

            error.reported_line = error
                .reported_line
                .or_else(|| Some(self.reported_line(&error.diagnostic)));

            return Err(error);
        }

        Ok(self.stream.into_ast(&self.lexer))
    }

    #[must_use]
    pub fn parse_unsafe(mut self) -> ParseOutput {
        self.stream = ParseStream::new_unsafe();

        let failure = self.parse_chunk().err();
        let mut errors = self.stream.diagnostics.take_errors();

        errors.extend(failure.map(|error| *error));

        let lexer_errors = self.stream.tokens.take_errors();

        ParseOutput {
            ast: self.stream.into_ast(&self.lexer),
            errors,
            lexer_errors,
        }
    }

    #[must_use]
    pub fn lexer(&self) -> &Lexer<'src> {
        &self.lexer
    }

    #[must_use]
    pub fn grammar(&self) -> &'static Grammar {
        self.grammar
    }

    #[must_use]
    pub fn source(&self) -> &'src str {
        self.lexer.source()
    }

    #[must_use]
    pub fn options(&self) -> &ParserOptions {
        &self.options
    }

    #[must_use]
    pub fn token(&self) -> &AnyToken {
        self.stream.tokens.current()
    }

    #[must_use]
    pub fn prev(&self) -> &AnyToken {
        self.stream.tokens.previous()
    }

    pub fn advance(&mut self) -> ParseResult<()> {
        self.lexer.advance(&mut self.stream.tokens)
    }

    #[must_use]
    pub fn is_preserving(&self) -> bool {
        self.options.preserve
    }

    #[must_use]
    pub fn should_parse_numbers_as_mono(&self) -> bool {
        self.options.mono_number_parsing
    }

    pub fn raise(&mut self, mut error: ParseError) -> ParseResult<()> {
        error.reported_line = Some(self.reported_line(&error.diagnostic));

        self.stream.diagnostics.report(error)
    }

    pub(crate) fn reported_line(&self, diagnostic: &Diagnostic) -> u32 {
        let offset = match diagnostic.reported_line() {
            ReportedLine::Statement => self
                .stream
                .statement
                .unwrap_or_else(|| self.token().span().start),
            ReportedLine::Token => {
                let span = self.token().span();
                let at_break = self.token().is_eol()
                    && self.source().as_bytes().get(span.start as usize) == Some(&b'\n');

                if at_break { span.start + 1 } else { span.end }
            }
            ReportedLine::End => {
                let end = u32::try_from(self.source().len()).unwrap_or(u32::MAX);

                return self.line(end) + 1;
            }
        };

        self.line(offset)
    }

    pub fn raise_at(&mut self, diagnostic: impl Into<Diagnostic>, span: Span) -> ParseResult<()> {
        self.raise(ParseError::from_parser(diagnostic, span))
    }

    pub fn report_at(&mut self, diagnostic: impl Into<Diagnostic>, span: Span) -> ParseResult<()> {
        let mut error = ParseError::from_parser(diagnostic, span);

        error.reported_line = Some(self.reported_line(&error.diagnostic));

        self.stream.diagnostics.report_global(error)
    }

    pub fn alloc(&mut self, kind: impl Into<NodeKind>, span: Span) -> NodeId {
        let id = self
            .stream
            .nodes
            .alloc(kind, span, self.stream.scopes.current());

        if self.combined_depth(self.stream.nodes.depth(id)) > MAX_NODE_DEPTH {
            self.stream.over_depth_span.get_or_insert(span);
        }

        id
    }

    fn combined_depth(&self, depth: usize) -> usize {
        depth + self.stream.blocks.depth()
    }

    pub(crate) fn check_depth(&mut self) -> ParseResult<()> {
        let span =
            self.stream.over_depth_span.take().or_else(|| {
                (self.stream.blocks.depth() > MAX_NODE_DEPTH).then(|| self.prev().span())
            });

        match span {
            Some(span) => self.raise_at(
                DepthDiagnostic::DepthLimit {
                    limit: MAX_NODE_DEPTH,
                },
                span,
            ),
            None => Ok(()),
        }
    }

    #[must_use]
    pub fn lines(&self) -> &LineIndex {
        self.lexer.lines()
    }

    #[must_use]
    pub fn line(&self, offset: u32) -> u32 {
        self.lines().line(offset)
    }

    #[must_use]
    pub fn span_from(&self, start: &Span) -> Span {
        start.to(&self.prev().span())
    }

    pub fn register_line_item(&mut self, id: NodeId) {
        self.stream.chunk_data.current_mut().line_items.push(id);
    }

    pub fn scope_data_mut(&mut self) -> &mut PendingScope {
        self.stream
            .scopes
            .current_mut()
            .expect("a scope is pending while parsing statements")
    }

    pub fn enter_scope(&mut self, scope: NodeId) {
        if self.stream.scopes.enter(scope, &mut self.stream.nodes) {
            self.stream.chunk_data.current_mut().scopes.push(scope);
        }
    }

    pub fn resume_scope(&mut self, scope: NodeId) {
        self.stream
            .scopes
            .resume_scope(scope, &mut self.stream.nodes);
    }

    pub fn exit_scope(&mut self) {
        self.stream.scopes.exit(&mut self.stream.nodes);
    }

    pub fn complete_top_block(&mut self, end: &Span) {
        if let Some(block) = self.stream.blocks.pop_inner() {
            block.complete(self, end);
        }
    }

    pub fn raise_unmatched_end(
        &mut self,
        end: impl Into<String>,
        block: &'static str,
    ) -> ParseResult<()> {
        let span = self.token().span();
        let diagnostic = BlockDiagnostic::UnmatchedEnd {
            end: end.into(),
            block,
        };

        self.raise_at(diagnostic, span)
    }

    pub fn raise_without_starter(&mut self, keyword: &'static str) -> ParseResult<()> {
        let span = self.token().span();

        self.raise_at(BlockDiagnostic::WithoutStarter(keyword), span)
    }
}
