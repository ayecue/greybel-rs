use greybel_symbols::names::{LOOP_INDEX_PREFIX, LOOP_INDEX_SUFFIX};

use super::loop_block::{LoopNode, PendingLoop};
use super::shortcut::parse_shortcut_statement;
use crate::ast::{
    AstNode, DumpContext, LiteralValue, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore,
    UnaryOp,
};
use crate::diagnostics::StatementDiagnostic;
use crate::dialect::SyntaxFeatures;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct};
use crate::parser::{Expect, KeywordStatement, Parser, groups};
use crate::position::Span;
use crate::syntax::greybel::{DestructuringAssignment, DestructuringExpansion};
use crate::syntax::{Assignment, Identifier, IdentifierKind};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct For {
    #[child]
    pub variable: NodeId,
    #[child]
    pub sequence: NodeId,
    #[child]
    #[body]
    pub body: NodeList,
    /// `for i, item in` or `for [x, y] in`: the names as written and how many statements at the start of `body`
    /// assign them.
    pub header: Option<ForHeader>,
}

/// The names of a Greybel `for` header: an index name (`i = __item_idx`) and/or a destructuring of the item, assigned
/// by the first `prelude` statements of the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForHeader {
    pub span: Span,
    pub prelude: usize,
}

impl AstNode for For {
    fn type_name(&self) -> &'static str {
        "ForGenericStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let type_name = self.type_name();
        let loc = ctx.location();
        let head = format!(
            "{} in {}",
            ctx.child(self.variable),
            ctx.child(self.sequence)
        );

        if self.body.is_empty() {
            format!("{type_name}[{loc}][{head}]")
        } else {
            format!("For[{loc}][{head}\n{}\n]", ctx.block(self.body))
        }
    }
}

impl LoopNode for For {
    const END: Keyword = Keyword::EndFor;
    const NAME: &'static str = Keyword::For.as_str();
}

#[derive(Debug, Clone, Copy)]
pub struct ForRule;

impl KeywordStatement for ForRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::For]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.advance()?;

        let scope = parser.stream.scopes.current();
        let start = parser.prev().span();
        let header_start = parser.token().span();
        let (variable, prelude) = if Self::has_names(parser) {
            Self::parse_names(parser)?
        } else {
            (
                Identifier::parse(parser, IdentifierKind::ForInVariable)?,
                Vec::new(),
            )
        };
        let header = (!prelude.is_empty()).then(|| ForHeader {
            span: header_start.to(&parser.prev().span()),
            prelude: prelude.len(),
        });

        parser.require(Expect::Keyword(Keyword::In), Some(start.start))?;

        let sequence = parser.parse_expr(None)?;

        if is_null(&parser.stream.nodes, sequence) {
            let span = parser.stream.nodes.get(sequence).span;

            parser.report_at(StatementDiagnostic::ForWithoutSequence, span)?;
        }

        let shortcut =
            parser.grammar().single_line_loops && !parser.is_at_any(groups::BLOCK_END_OF_LINE);
        let depth = parser.stream.blocks.depth();
        let body = if shortcut {
            let item = parse_shortcut_statement(parser)?;
            let mut body = prelude.clone();

            body.push(item);

            parser.stream.nodes.push_list(&body)
        } else {
            NodeList::EMPTY
        };
        let node = parser.alloc(
            For {
                variable,
                sequence,
                body,
                header,
            },
            parser.span_from(&start),
        );

        if let Some(scope) = scope.and_then(|s| parser.stream.scopes.pending_mut(s)) {
            scope.definitions.push(node);
        }

        if shortcut {
            parser.emit_statement(depth, node, true);
        } else {
            PendingLoop::<For>::open(parser, node);
            parser.stream.blocks.top_mut().body_mut().extend(prelude);
        }

        Ok(())
    }
}

impl ForRule {
    /// The loop variable name for the items of `for [x, y] in`.
    pub const ITEM: &'static str = "__e";

    /// Whether the header is `i, item`, `[x, y]` or `{key, value}` rather than one name.
    fn has_names(parser: &Parser<'_>) -> bool {
        let token = parser.token();

        if token.is_punct(Punct::LBracket) || token.is_punct(Punct::LBrace) {
            return parser
                .grammar()
                .allows(SyntaxFeatures::DESTRUCTURING_ASSIGNMENTS);
        }

        parser.grammar().allows(SyntaxFeatures::FOR_INDEX)
            && token.as_identifier().is_some()
            && parser.source()[token.span().end as usize..]
                .trim_start_matches([' ', '\t'])
                .starts_with(',')
    }

    /// `[index,] name` or `[index,] [x, y]`: the loop variable and the statements at the start of the body that
    /// assign the index and the destructured names.
    fn parse_names(parser: &mut Parser<'_>) -> ParseResult<(NodeId, Vec<NodeId>)> {
        let origin = parser.token().span().start;
        let index = if parser.token().as_identifier().is_some() {
            let after = parser.source()[parser.token().span().end as usize..]
                .trim_start_matches([' ', '\t']);

            if after.starts_with(',') {
                let index = Identifier::parse(parser, IdentifierKind::Variable)?;

                parser.require(Expect::Punct(Punct::Comma), None)?;

                Some(index)
            } else {
                None
            }
        } else {
            None
        };
        let target = if parser
            .grammar()
            .allows(SyntaxFeatures::DESTRUCTURING_ASSIGNMENTS)
            && (parser.token().is_punct(Punct::LBracket) || parser.token().is_punct(Punct::LBrace))
        {
            Some(DestructuringAssignment::parse_target(parser)?)
        } else {
            None
        };
        let variable = match target {
            Some(_) => {
                let name = parser.stream.nodes.insert_atom(Self::ITEM);
                let variable = parser.alloc(
                    Identifier {
                        name,
                        kind: IdentifierKind::ForInVariable,
                    },
                    Span::empty(origin),
                );

                parser.scope_data_mut().namespaces.push(variable);

                variable
            }
            None => Identifier::parse(parser, IdentifierKind::ForInVariable)?,
        };
        let mut prelude = Vec::new();

        if let Some(index) = index {
            let item = match &parser.stream.nodes.get(variable).kind {
                NodeKind::Identifier(identifier) => {
                    parser.stream.nodes.resolve(identifier.name).to_owned()
                }
                _ => String::new(),
            };
            let counter = parser
                .stream
                .nodes
                .insert_atom(&format!("{LOOP_INDEX_PREFIX}{item}{LOOP_INDEX_SUFFIX}"));
            let span = parser.stream.nodes.get(index).span;
            let value = parser.alloc(
                Identifier {
                    name: counter,
                    kind: IdentifierKind::Variable,
                },
                Span::empty(origin),
            );
            let assignment = parser.alloc(
                Assignment {
                    variable: index,
                    init: value,
                    shorthand: false,
                    declaration: None,
                },
                span,
            );

            parser.scope_data_mut().definitions.push(assignment);
            prelude.push(assignment);
        }

        if let Some(target) = target {
            let item = parser.stream.nodes.clone_subtree(variable);

            prelude.extend(DestructuringExpansion::expand(
                parser, &target, item, origin,
            )?);
        }

        Ok((variable, prelude))
    }
}

fn is_null(nodes: &NodeStore, id: crate::ast::NodeId) -> bool {
    match &nodes.get(id).kind {
        NodeKind::Literal(literal) => literal.value == LiteralValue::Nil,
        NodeKind::Parenthesis(parenthesis) => is_null(nodes, parenthesis.expression),
        NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf => {
            is_null(nodes, unary.argument)
        }
        _ => false,
    }
}
