use greybel_symbols::names::{EXPORTS, MODULE};

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::parser::Parser;
use crate::position::Span;
use crate::syntax::{
    Assignment, Identifier, IdentifierKind, Map, Member, Unary, UnaryOp, parse_expression_statement,
};

/// `export name = value` at the top of a file loaded with `#import`. `statements` are the assignment, then
/// `module.exports.name = @name`; the first export of a file starts with `module.exports = {}`. Builds write those
/// statements.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Export {
    #[child]
    pub statements: NodeList,
    /// The assignment as written, for dev mode.
    pub assignment: NodeId,
}

impl AstNode for Export {
    fn type_name(&self) -> &'static str {
        "ExportStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "ExportStatement[{}][\n{}\n]",
            ctx.location(),
            ctx.block(self.statements)
        )
    }
}

impl Export {
    pub(crate) fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let start = parser.token().span();

        if parser.stream.blocks().depth() > 0 {
            parser.raise_at(GreybelDiagnostic::NestedExport, start)?;
        }

        parser.advance()?;

        let assignment = parse_expression_statement(parser)?;
        let variable = match &parser.stream.nodes.get(assignment).kind {
            NodeKind::Assignment(assignment) if !assignment.shorthand => Some(assignment.variable),
            _ => None,
        }
        .filter(|&variable| {
            matches!(
                parser.stream.nodes.get(variable).kind,
                NodeKind::Identifier(_)
            )
        });
        let span = parser.span_from(&start);
        let at = Span::empty(start.start);
        let Some(variable) = variable else {
            parser.raise_at(GreybelDiagnostic::ExportWithoutAssignment, span)?;

            return Ok(assignment);
        };
        let first = !parser
            .stream
            .blocks()
            .root_statements()
            .iter()
            .any(|&statement| {
                matches!(parser.stream.nodes.get(statement).kind, NodeKind::Export(_))
            });
        let mut statements = Vec::new();

        if first {
            let target = Self::exports(parser, at);
            let init = parser.alloc(Map::default(), at);

            statements.push(parser.alloc(
                Assignment {
                    variable: target,
                    init,
                    shorthand: false,
                    declaration: None,
                },
                at,
            ));
        }

        statements.push(assignment);

        let exports = Self::exports(parser, at);
        let name = match &parser.stream.nodes.get(variable).kind {
            NodeKind::Identifier(identifier) => identifier.name,
            _ => unreachable!("the variable is an identifier"),
        };
        let key = parser.alloc(
            Identifier {
                name,
                kind: IdentifierKind::Property,
            },
            at,
        );
        let target = parser.alloc(
            Member {
                base: exports,
                identifier: key,
                optional: false,
            },
            at,
        );
        let value = parser.stream.nodes.clone_subtree(variable);
        let reference = parser.alloc(
            Unary {
                operator: UnaryOp::AddressOf,
                argument: value,
            },
            at,
        );

        statements.push(parser.alloc(
            Assignment {
                variable: target,
                init: reference,
                shorthand: false,
                declaration: None,
            },
            at,
        ));

        let statements = parser.stream.nodes.push_list(&statements);

        Ok(parser.alloc(
            Export {
                statements,
                assignment,
            },
            span,
        ))
    }

    /// `module.exports`.
    fn exports(parser: &mut Parser<'_>, at: Span) -> NodeId {
        let module = parser.stream.nodes.insert_atom(MODULE);
        let exports = parser.stream.nodes.insert_atom(EXPORTS);
        let base = parser.alloc(
            Identifier {
                name: module,
                kind: IdentifierKind::Variable,
            },
            at,
        );
        let identifier = parser.alloc(
            Identifier {
                name: exports,
                kind: IdentifierKind::Property,
            },
            at,
        );

        parser.alloc(
            Member {
                base,
                identifier,
                optional: false,
            },
            at,
        )
    }
}
