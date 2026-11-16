use crate::ast::{
    AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore, ScopeData,
};
use crate::diagnostics::{FunctionDiagnostic, GreybelDiagnostic};
use crate::dialect::SyntaxFeatures;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct, TokenKind};
use crate::parser::{Expect, KeywordStatement, Parser, PendingBlock, groups};
use crate::position::Span;
use crate::syntax::{Assignment, Identifier, IdentifierKind, Return, Unknown};

#[derive(Debug, Clone, Default, PartialEq, NodeFieldContainer)]
pub struct Function {
    #[child]
    pub parameters: NodeList,
    #[child]
    #[body]
    pub body: NodeList,
    #[scope]
    pub scope: ScopeData,
    pub assignment: Option<NodeId>,
    /// Written as `(x) => ...`; builds write it as `function(x)`.
    pub arrow: Option<ArrowBody>,
}

/// The body of a `(x) => ...` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowBody {
    /// `(x) => x * 2`: the body is `return` of the expression.
    Expression,
    /// `(x) =>` at the end of a line, with the body on the following lines up to `end function`.
    Block,
}

impl AstNode for Function {
    fn type_name(&self) -> &'static str {
        "FunctionDeclaration"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let loc = ctx.location();
        let arguments = ctx.list(self.parameters);

        if self.body.is_empty() {
            format!("{type_name}[{loc}][{arguments}]")
        } else {
            format!(
                "Function[{loc}][{arguments} =>\n{}\n]",
                ctx.block(self.body)
            )
        }
    }
}

pub struct PendingFunction {
    pub node: NodeId,
    pub base: Option<NodeId>,
    pub body: Vec<NodeId>,
}

impl PendingBlock for PendingFunction {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.body
    }

    fn name(&self) -> &'static str {
        Keyword::Function.as_str()
    }

    fn start(&self, nodes: &NodeStore) -> u32 {
        nodes.get(self.node).span.start
    }

    fn complete(self: Box<Self>, parser: &mut Parser<'_>, end: &Span) {
        parser.stream.nodes.close_block(self.node, self.body, end);
        if let Some(base) = self.base {
            let span = parser.stream.nodes.get(self.node).span;

            parser.stream.nodes.close(base, &span);
            parser.register_line_item(base);
        }

        parser.register_line_item(self.node);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndFunctionRule;

impl KeywordStatement for EndFunctionRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::EndFunction]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.parse_block_end(|parser| {
            if parser.grammar().checks_open_blocks_per_function
                && !parser.stream.blocks.top_is::<PendingFunction>()
                && parser.stream.blocks.contains::<PendingFunction>()
            {
                let line = parser.line(parser.prev().span().start);

                while !parser.stream.blocks.top_is::<PendingFunction>() {
                    let block = parser
                        .stream
                        .blocks
                        .pop_inner()
                        .expect("a block in the function");

                    parser.close_open_block(block, Some(line))?;
                }
            }

            if !parser.stream.blocks.top_is::<PendingFunction>() {
                return parser.raise_without_starter(Keyword::EndFunction.as_str());
            }

            parser.exit_scope();

            let end = parser.prev().span();

            parser.complete_top_block(&end);

            Ok(())
        })
    }
}

impl Function {
    /// How far [`Self::is_arrow_at`] looks for the `)` of the parameters, so a line of parentheses isn't scanned
    /// once per `(`.
    pub const MAX_ARROW_PARAMETERS_LEN: usize = 256;

    #[must_use]
    pub fn is_literal_default(nodes: &NodeStore, default: NodeId) -> bool {
        match &nodes.get(default).kind {
            NodeKind::Literal(_) => true,
            NodeKind::Parenthesis(parenthesis) => {
                Self::is_literal_default(nodes, parenthesis.expression)
            }
            _ => false,
        }
    }

    /// Whether the `(` at `start` opens the parameters of a `(x) =>` function: its `)` on the same line, within
    /// [`Self::MAX_ARROW_PARAMETERS_LEN`] bytes, is followed by `=>`.
    #[must_use]
    pub fn is_arrow_at(source: &str, start: u32) -> bool {
        let tail = &source.as_bytes()[start as usize..];
        let bytes = &tail[..tail.len().min(Self::MAX_ARROW_PARAMETERS_LEN)];
        let mut depth = 0_usize;
        let mut index = 0;

        while index < bytes.len() {
            match bytes[index] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;

                    if depth == 0 {
                        let after = &source[start as usize + index + 1..];

                        return after
                            .trim_start_matches([' ', '\t'])
                            .starts_with(Punct::Arrow.as_str());
                    }
                }
                b'"' => {
                    index += 1;

                    while index < bytes.len() && bytes[index] != b'"' {
                        if bytes[index] == b'\n' {
                            return false;
                        }

                        index += 1;
                    }
                }
                b'\n' | b'\r' => return false,
                _ => {}
            }

            index += 1;
        }

        false
    }

    /// `(x) => expression`, or `(x) =>` at the end of a line with a body up to `end function`.
    pub fn parse_arrow(parser: &mut Parser<'_>, base: Option<NodeId>) -> ParseResult<NodeId> {
        let start = parser.token().span();
        let function = parser.alloc(
            Function {
                assignment: parser.stream.scopes.assignment(),
                ..Function::default()
            },
            start.open(),
        );

        parser.enter_scope(function);

        let mut parameters = parser.stream.nodes.list_buffer();

        parser.require(Expect::Punct(Punct::LParen), Some(start.start))?;
        while !parser.is_at_any(groups::PARAMETERS_END) {
            parameters.push(Self::parse_parameter(parser)?);
            if parser.token().is_punct(Punct::RParen) {
                break;
            }

            parser.require(Expect::Punct(Punct::Comma), Some(start.start))?;
        }

        parser.require(Expect::Punct(Punct::RParen), Some(start.start))?;
        parser.require(Expect::Punct(Punct::Arrow), Some(start.start))?;

        let parameters = parser.stream.nodes.finish_list(parameters);
        let block = parser.is_at_any(groups::BLOCK_END_OF_LINE);

        if let NodeKind::Function(node) = &mut parser.stream.nodes.get_mut(function).kind {
            node.parameters = parameters;
            node.arrow = Some(if block {
                ArrowBody::Block
            } else {
                ArrowBody::Expression
            });
        }

        if block {
            let body = parser.stream.nodes.list_buffer();

            parser.stream.blocks.push(PendingFunction {
                node: function,
                base,
                body,
            });

            return Ok(function);
        }

        let return_start = parser.token().span();
        let argument = parser.parse_expr(None)?;
        let return_span = parser.span_from(&return_start);
        let statement = parser.alloc(
            Return {
                argument: Some(argument),
            },
            return_span,
        );
        let body = parser.stream.nodes.push_list(&[statement]);

        parser.exit_scope();

        let end = parser.prev().span();
        let node = parser.stream.nodes.get_mut(function);

        node.span = start.to(&end);

        if let NodeKind::Function(function) = &mut node.kind {
            function.body = body;
        }

        Ok(function)
    }

    pub fn parse(parser: &mut Parser<'_>, base: Option<NodeId>) -> ParseResult<NodeId> {
        parser.advance()?;

        let start = parser.prev().span();
        let function = parser.alloc(
            Function {
                assignment: parser.stream.scopes.assignment(),
                ..Function::default()
            },
            start.open(),
        );

        parser.enter_scope(function);
        Self::parse_templates(parser, function)?;

        let mut parameters = parser.stream.nodes.list_buffer();

        if !parser.is_at_any(groups::STATEMENT_END) {
            parser.require(Expect::Punct(Punct::LParen), Some(start.start))?;
            while !parser.is_at_any(groups::PARAMETERS_END) {
                parameters.push(Self::parse_parameter(parser)?);
                if parser.token().is_punct(Punct::RParen) {
                    break;
                }

                parser.require(Expect::Punct(Punct::Comma), Some(start.start))?;
            }

            parser.require(Expect::Punct(Punct::RParen), Some(start.start))?;
            Self::parse_return_type(parser, function)?;
        }

        let parameters = parser.stream.nodes.finish_list(parameters);

        if let NodeKind::Function(node) = &mut parser.stream.nodes.get_mut(function).kind {
            node.parameters = parameters;
        }

        let body = parser.stream.nodes.list_buffer();

        parser.stream.blocks.push(PendingFunction {
            node: function,
            base,
            body,
        });

        Ok(function)
    }

    /// `name(parameters)` in a class body, after the name: a function assigned by `base`, with its body up to
    /// `end function`.
    pub(crate) fn parse_method(parser: &mut Parser<'_>, base: NodeId) -> ParseResult<NodeId> {
        let start = parser.token().span();
        let function = parser.alloc(
            Function {
                assignment: Some(base),
                ..Function::default()
            },
            start.open(),
        );

        parser.enter_scope(function);

        let mut parameters = parser.stream.nodes.list_buffer();

        parser.require(Expect::Punct(Punct::LParen), Some(start.start))?;
        while !parser.is_at_any(groups::PARAMETERS_END) {
            parameters.push(Self::parse_parameter(parser)?);
            if parser.token().is_punct(Punct::RParen) {
                break;
            }

            parser.require(Expect::Punct(Punct::Comma), Some(start.start))?;
        }

        parser.require(Expect::Punct(Punct::RParen), Some(start.start))?;
        Self::parse_return_type(parser, function)?;

        let parameters = parser.stream.nodes.finish_list(parameters);

        if let NodeKind::Function(node) = &mut parser.stream.nodes.get_mut(function).kind {
            node.parameters = parameters;
        }

        let body = parser.stream.nodes.list_buffer();

        parser.stream.blocks.push(PendingFunction {
            node: function,
            base: Some(base),
            body,
        });

        Ok(function)
    }

    /// `<T, U>` after `function`, with types on.
    fn parse_templates(parser: &mut Parser<'_>, function: NodeId) -> ParseResult<()> {
        if !(parser.grammar().allows(SyntaxFeatures::TYPES) && parser.token().is_punct(Punct::Less))
        {
            return Ok(());
        }

        parser.advance()?;

        let mut names = Vec::new();

        loop {
            if parser.token().as_identifier().is_none() {
                let span = parser.token().span();

                return parser.raise_at(GreybelDiagnostic::ExpectedType, span);
            }

            names.push(parser.token().span());
            parser.advance()?;

            if !parser.consume(Expect::Punct(Punct::Comma))? {
                break;
            }
        }

        parser.require(Expect::Punct(Punct::Greater), None)?;
        parser.stream.type_syntax.set_templates(function, names);

        Ok(())
    }

    /// `: T` after the parameters, with types on.
    fn parse_return_type(parser: &mut Parser<'_>, function: NodeId) -> ParseResult<()> {
        if !Self::is_at_type(parser) {
            return Ok(());
        }

        let start = parser.token().span().end;

        if let Some(ty) = parser.parse_type_text(start)? {
            parser.stream.type_syntax.set_returns(function, ty);
        }

        Ok(())
    }

    /// A `:` that starts a type, with types on.
    fn is_at_type(parser: &Parser<'_>) -> bool {
        parser.grammar().allows(SyntaxFeatures::TYPES)
            && parser.token().kind() == TokenKind::SliceOperator
    }

    fn parse_parameter(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let parameter = Identifier::parse(parser, IdentifierKind::Argument)?;
        let parameter_span = parser.stream.nodes.get(parameter).span;

        if Self::is_at_type(parser) {
            let start = parser.token().span().end;

            if let Some(ty) = parser.parse_type_text(start)? {
                parser.stream.type_syntax.set_parameter(parameter, ty);
            }
        }

        if !parser.consume(Expect::Punct(Punct::Assign))? {
            let span = parser.span_from(&parameter_span);
            let unknown = parser.alloc(Unknown, span);
            let assignment = parser.alloc(
                Assignment {
                    variable: parameter,
                    init: unknown,
                    shorthand: false,
                    declaration: None,
                },
                span,
            );

            parser.scope_data_mut().definitions.push(assignment);

            return Ok(parameter);
        }

        let default = parser.parse_expr(None)?;

        if !parser
            .grammar()
            .accepts_default(&parser.stream.nodes, default)
        {
            let span = parameter_span.to(&parser.token().span());

            parser.report_at(FunctionDiagnostic::DefaultNotLiteral, span)?;
        }

        let assignment = parser.alloc(
            Assignment {
                variable: parameter,
                init: default,
                shorthand: false,
                declaration: None,
            },
            parser.span_from(&parameter_span),
        );

        parser.scope_data_mut().definitions.push(assignment);

        Ok(assignment)
    }
}
