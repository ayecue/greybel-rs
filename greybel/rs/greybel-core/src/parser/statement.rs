use super::state::PendingBlock;
use super::{Expect, Parser, groups};
use crate::ast::{NodeId, NodeList, ScopeData};
use crate::diagnostics::{BlockDiagnostic, ExpectationDiagnostic, KeywordDiagnostic};
use crate::error::{ParseError, ParseResult};
use crate::lexer::Keyword;
use crate::position::Span;
use crate::syntax::{Chunk, PendingChunk, parse_expression_statement};

impl Parser<'_> {
    pub(crate) fn parse_chunk(&mut self) -> ParseResult<()> {
        let source_len = self.lexer.source().len();

        self.stream.nodes.reserve_for_source(source_len);
        self.advance()?;

        let chunk = self.alloc(
            Chunk {
                body: NodeList::EMPTY,
                scope: ScopeData::default(),
            },
            self.token().span().open(),
        );

        self.stream.root = Some(chunk);

        let body = self.stream.nodes.list_buffer();

        self.stream.blocks.push(PendingChunk { node: chunk, body });
        self.enter_scope(chunk);

        while !self.token().is_eof() {
            self.consume_noop_statement()?;
            if self.token().is_eof() {
                break;
            }

            self.parse_statement()?;
        }

        self.flush_pending_comments();
        self.finish_open_blocks()?;
        self.stream.scopes.close_all(&mut self.stream.nodes);

        let end = self.token().span();
        let chunk_block = self.stream.blocks.pop_root().expect("chunk is open");

        chunk_block.complete(self, &end);

        Ok(())
    }

    pub(crate) fn parse_statement_inner(&mut self) -> ParseResult<()> {
        if let Some(keyword) = self.token().keyword()
            && keyword != Keyword::Not
        {
            match self.statement_rule(keyword) {
                Some(rule) => rule.parse(self, keyword)?,
                None => self.raise_unexpected_keyword()?,
            }

            if std::mem::take(&mut self.stream.line_continues) {
                return Ok(());
            }

            return self.require_statement_end();
        }

        let depth = self.stream.blocks.depth();
        let item = parse_expression_statement(self)?;
        let closed = self.stream.nodes.get(item).span.is_closed();

        self.emit_statement(depth, item, closed);

        self.require_statement_end()
    }

    fn require_statement_end(&mut self) -> ParseResult<()> {
        if self.is_at_any(groups::STATEMENT_END)
            || self.prev().is_eol()
            || self.stream.diagnostics.has_statement_errors()
        {
            return Ok(());
        }

        let diagnostic = ExpectationDiagnostic::Token {
            got: self.token().describe(self.source()),
            expectation: Expect::Eol.describe(),
        };
        let span = self.token().span();

        self.raise_at(diagnostic, span)
    }

    pub fn emit_statement(&mut self, depth: usize, item: NodeId, register_line_item: bool) {
        if register_line_item {
            self.register_line_item(item);
        }

        self.attach_pending_comments(item);
        self.stream.blocks.push_statement_at(depth, item);
    }

    pub fn parse_block_end(
        &mut self,
        close: impl FnOnce(&mut Self) -> ParseResult<()>,
    ) -> ParseResult<()> {
        self.advance()?;
        self.flush_pending_comments();
        close(self)?;

        self.consume_end_trailing_comment()
    }

    fn finish_open_blocks(&mut self) -> ParseResult<()> {
        while let Some(block) = self.stream.blocks.pop_inner() {
            self.close_open_block(block, None)?;
        }

        Ok(())
    }

    pub(crate) fn close_open_block(
        &mut self,
        block: Box<dyn PendingBlock>,
        line: Option<u32>,
    ) -> ParseResult<()> {
        let start = block.start(&self.stream.nodes);
        let name = block.name();
        let awaits_end = block.awaits_end(&self.stream.nodes)
            || (self.grammar().checks_open_blocks_per_function && name == Keyword::If.as_str());
        let inside = self.stream.blocks.top_inner().map(|parent| {
            let parent_start = parent.start(&self.stream.nodes);

            (parent.name(), self.line(parent_start))
        });
        let error = ParseError::from_parser(
            BlockDiagnostic::MissingEnd {
                block: name,
                awaits_end,
                inside,
            },
            Span::empty(start),
        );
        let error = ParseError {
            reported_line: Some(line.unwrap_or_else(|| self.reported_line(&error.diagnostic))),
            ..error
        };
        let end = self.prev().span();

        block.complete(self, &end);

        self.stream.diagnostics.report_global(error)
    }

    pub(crate) fn raise_unexpected_keyword(&mut self) -> ParseResult<()> {
        let diagnostic =
            KeywordDiagnostic::Unexpected(self.token().value(self.source()).into_owned());
        let span = self.token().span();

        self.raise_at(diagnostic, span)
    }
}
