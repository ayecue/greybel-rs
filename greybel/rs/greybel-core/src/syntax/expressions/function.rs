use crate::ast::{
    AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore, ScopeData,
};
use crate::diagnostics::FunctionDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct};
use crate::parser::{Expect, KeywordStatement, Parser, PendingBlock, groups};
use crate::position::Span;
use crate::syntax::{Assignment, Identifier, IdentifierKind, Invalid, Unknown};

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

    fn parse_parameter(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let parameter = Identifier::parse(parser, IdentifierKind::Argument)?;
        let parameter_span = parser.stream.nodes.get(parameter).span;

        if !parser.consume(Expect::Punct(Punct::Assign))? {
            let span = parser.span_from(&parameter_span);
            let unknown = parser.alloc(Unknown, span);
            let assignment = parser.alloc(
                Assignment {
                    variable: parameter,
                    init: unknown,
                    shorthand: false,
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

            parser.raise_at(FunctionDiagnostic::DefaultNotLiteral, span)?;

            return Ok(parser
                .stream
                .nodes
                .alloc(Invalid, parser.span_from(&parameter_span), None));
        }

        let assignment = parser.alloc(
            Assignment {
                variable: parameter,
                init: default,
                shorthand: false,
            },
            parser.span_from(&parameter_span),
        );

        parser.scope_data_mut().definitions.push(assignment);

        Ok(assignment)
    }
}
