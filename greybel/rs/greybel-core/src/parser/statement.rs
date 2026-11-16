use greybel_symbols::contextual_keywords::{
    ASSERT, CASE, CLASS, CONST, ENUM, EXPORT, SWITCH, TYPE,
};

use super::state::PendingBlock;
use super::{Expect, Parser, groups};
use crate::ast::{NodeId, NodeKind, NodeList, ScopeData};
use crate::diagnostics::{
    BlockDiagnostic, ExpectationDiagnostic, GreybelDiagnostic, KeywordDiagnostic,
};
use crate::dialect::SyntaxFeatures;
use crate::error::{ParseError, ParseResult};
use crate::lexer::{Keyword, Punct, is_identifier_start};
use crate::position::Span;
use crate::syntax::greybel::{
    Assert, Class, DestructuringAssignment, Enum, Export, Switch, TypeStatement,
};
use crate::syntax::{
    Assignment, Chunk, Declaration, Identifier, IdentifierKind, PendingChunk,
    parse_expression_statement,
};

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
        self.stream.scopes.finish_all(&mut self.stream.nodes);

        let end = self.token().span();
        let chunk_block = self.stream.blocks.pop_root().expect("chunk is open");

        chunk_block.complete(self, &end);

        Ok(())
    }

    pub(crate) fn parse_statement_inner(&mut self) -> ParseResult<()> {
        if self.is_in_class() && !self.token().is_keyword(Keyword::EndClass) {
            Class::parse_member(self)?;

            return self.require_statement_end();
        }

        if self.token().is_keyword(Keyword::Else) && self.is_in_switch() {
            Switch::parse_fallback(self)?;

            return self.require_statement_end();
        }

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

        if self.grammar().allows(SyntaxFeatures::ENUM) && self.is_at_named_statement(ENUM) {
            let item = Enum::parse(self, false)?;

            self.emit_statement(depth, item, true);

            return self.require_statement_end();
        }

        if self.grammar().allows(SyntaxFeatures::ENUM) && self.is_at_const_enum() {
            let start = self.token().span().start;

            self.advance()?;

            let item = Enum::parse(self, true)?;

            self.stream.nodes.get_mut(item).span.start = start;

            self.emit_statement(depth, item, true);

            return self.require_statement_end();
        }

        if self.grammar().allows(SyntaxFeatures::ASSERT) && self.is_at_word_statement(ASSERT) {
            let item = Assert::parse(self)?;

            self.emit_statement(depth, item, true);

            return self.require_statement_end();
        }

        if self.grammar().allows(SyntaxFeatures::EXPORT) && self.is_at_named_statement(EXPORT) {
            let item = Export::parse(self)?;

            self.emit_statement(depth, item, true);

            return self.require_statement_end();
        }

        if self.grammar().allows(SyntaxFeatures::TYPES) && self.is_at_named_statement(TYPE) {
            let item = TypeStatement::parse(self)?;

            self.emit_statement(depth, item, true);

            return self.require_statement_end();
        }

        if self.grammar().allows(SyntaxFeatures::CLASSES) && self.is_at_named_statement(CLASS) {
            Class::open(self)?;

            return self.require_statement_end();
        }

        if self.grammar().allows(SyntaxFeatures::SWITCH) && self.is_at_word_statement(SWITCH) {
            Switch::open(self)?;

            return self.require_statement_end();
        }

        if self.is_in_switch() && self.is_at_word_statement(CASE) {
            Switch::parse_case(self)?;

            return self.require_statement_end();
        }

        let declaration = self.declaration_prefix()?;
        let with_type = match declaration {
            Some(_) => self.parse_typed_declaration()?,
            None => None,
        };
        let item = if let Some(item) = with_type {
            item
        } else if self.is_at_destructuring_assignment() {
            DestructuringAssignment::parse(self)?
        } else {
            parse_expression_statement(self)?
        };

        if let Some((declaration, keyword)) = declaration {
            self.declare(item, declaration, keyword)?;
        }

        let closed = self.stream.nodes.get(item).span.is_closed();

        self.emit_statement(depth, item, closed);

        self.require_statement_end()
    }

    /// Consumes `let` or `const` when a name follows it on the line, as in `let count = 0`. Otherwise `let` and
    /// `const` are names, as in `let = 1`.
    fn declaration_prefix(&mut self) -> ParseResult<Option<(Declaration, Span)>> {
        if !self.grammar().allows(SyntaxFeatures::DECLARATIONS) {
            return Ok(None);
        }

        let token = self.token();
        let Some(declaration) = token
            .as_identifier()
            .and_then(|identifier| Declaration::parse(identifier.name(self.source())))
        else {
            return Ok(None);
        };
        let keyword = token.span();
        let after = self.source()[keyword.end as usize..].trim_start_matches([' ', '\t']);
        let destructures = self
            .grammar()
            .allows(SyntaxFeatures::DESTRUCTURING_ASSIGNMENTS)
            && after.starts_with(['[', '{']);

        if !self.is_followed_by_name(keyword) && !destructures {
            return Ok(None);
        }

        self.advance()?;

        Ok(Some((declaration, keyword)))
    }

    /// `name: T = value` after `let` or `const`, with types on.
    fn parse_typed_declaration(&mut self) -> ParseResult<Option<NodeId>> {
        if !self.grammar().allows(SyntaxFeatures::TYPES) || self.token().as_identifier().is_none() {
            return Ok(None);
        }

        let span = self.token().span();
        let after = self.source()[span.end as usize..].trim_start_matches([' ', '\t']);

        if !after.starts_with(':') {
            return Ok(None);
        }

        let identifier = Identifier::parse(self, IdentifierKind::Variable)?;
        let start = self.token().span().end;

        if let Some(ty) = self.parse_type_text(start)? {
            self.stream.type_syntax.set_variable(identifier, ty);
        }

        if !self.token().is_punct(Punct::Assign) {
            self.require(Expect::Punct(Punct::Assign), None)?;

            return Ok(Some(identifier));
        }

        Assignment::parse(self, identifier, span).map(Some)
    }

    /// `const enum Name`.
    fn is_at_const_enum(&self) -> bool {
        let token = self.token();
        let is_const = token
            .as_identifier()
            .is_some_and(|identifier| identifier.name(self.source()) == CONST);
        let after = self.source()[token.span().end as usize..].trim_start_matches([' ', '\t']);

        is_const
            && after.strip_prefix(ENUM).is_some_and(|tail| {
                tail.starts_with([' ', '\t'])
                    && tail
                        .trim_start_matches([' ', '\t'])
                        .starts_with(|character: char| {
                            character.is_ascii_alphabetic() || character == '_'
                        })
            })
    }

    /// Whether the current token is the word `word` followed by a name on the same line, as in `enum Color`.
    fn is_at_named_statement(&self, word: &str) -> bool {
        let token = self.token();

        token
            .as_identifier()
            .is_some_and(|identifier| identifier.name(self.source()) == word)
            && self.is_followed_by_name(token.span())
    }

    /// `word` (such as `assert` or `switch`) followed by a space and an expression, not an assignment or member
    /// access of a variable named `word` (`assert = f`, `assert.x`).
    fn is_at_word_statement(&self, word: &str) -> bool {
        let token = self.token();
        let is_word = token
            .as_identifier()
            .is_some_and(|identifier| identifier.name(self.source()) == word);

        if !is_word {
            return false;
        }

        let after = &self.source()[token.span().end as usize..];
        let condition = after.trim_start_matches([' ', '\t']);
        let assigns = ["+=", "-=", "*=", "/=", "%=", "^="]
            .iter()
            .any(|operator| condition.starts_with(operator))
            || (condition.starts_with('=') && !condition.starts_with("=="));
        let continues = condition.is_empty() || condition.starts_with(['\n', '\r', ';', '.', '[']);

        condition.len() < after.len() && !assigns && !continues
    }

    fn is_followed_by_name(&self, span: Span) -> bool {
        let after = &self.source().as_bytes()[span.end as usize..];
        let spaces = after
            .iter()
            .take_while(|&&byte| byte == b' ' || byte == b'\t')
            .count();

        spaces > 0
            && after
                .get(spaces)
                .is_some_and(|&byte| is_identifier_start(byte))
    }

    fn is_at_destructuring_assignment(&self) -> bool {
        let token = self.token();

        self.grammar()
            .allows(SyntaxFeatures::DESTRUCTURING_ASSIGNMENTS)
            && (token.is_punct(Punct::LBracket) || token.is_punct(Punct::LBrace))
            && DestructuringAssignment::is_at(self.source(), token.span().start)
    }

    /// Marks the assignments of a destructuring assignment with `declaration`, except its temporaries.
    fn declare_destructuring_assignment(&mut self, statements: NodeList, declaration: Declaration) {
        for statement in self.stream.nodes.list(statements).to_vec() {
            let NodeKind::Assignment(assignment) = &self.stream.nodes.get(statement).kind else {
                continue;
            };
            let temporary = match &self.stream.nodes.get(assignment.variable).kind {
                NodeKind::Identifier(identifier) => self
                    .stream
                    .nodes
                    .resolve(identifier.name)
                    .starts_with(DestructuringAssignment::TEMPORARY_PREFIX),
                _ => true,
            };

            if !temporary
                && let NodeKind::Assignment(assignment) =
                    &mut self.stream.nodes.get_mut(statement).kind
            {
                assignment.declaration = Some(declaration);
            }
        }
    }

    /// Marks the assignment `item` with `declaration`, starting it at the keyword.
    fn declare(
        &mut self,
        item: NodeId,
        declaration: Declaration,
        keyword: Span,
    ) -> ParseResult<()> {
        if let NodeKind::DestructuringAssignment(assignment) = &self.stream.nodes.get(item).kind {
            let statements = assignment.statements;

            self.declare_destructuring_assignment(statements, declaration);

            let node = self.stream.nodes.get_mut(item);

            node.span = keyword.to(&node.span);

            return Ok(());
        }

        let variable = match &self.stream.nodes.get(item).kind {
            NodeKind::Assignment(assignment) if !assignment.shorthand => Some(assignment.variable),
            _ => None,
        };
        let is_named_assignment = variable.is_some_and(|variable| {
            matches!(
                self.stream.nodes.get(variable).kind,
                NodeKind::Identifier(_)
            )
        });
        let node = self.stream.nodes.get_mut(item);

        if !is_named_assignment {
            let span = keyword.to(&node.span);

            return self.raise_at(
                GreybelDiagnostic::DeclarationWithoutAssignment {
                    keyword: declaration.as_str(),
                },
                span,
            );
        }

        node.span = keyword.to(&node.span);

        if let NodeKind::Assignment(assignment) = &mut node.kind {
            assignment.declaration = Some(declaration);
        }

        Ok(())
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
