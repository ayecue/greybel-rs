use rustc_hash::FxHashSet;

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, TextSpan};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct};
use crate::parser::{ExprContext, Parser};
use crate::position::Span;
use crate::syntax::{Assignment, Literal, LiteralValue, Map, MapEntry};

/// `enum Color` with one `Name [= value]` per line, then `end enum`. `assignment` is the
/// `Color = {"Red": 0, ...}` it stands for; builds write that assignment.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Enum {
    #[child]
    pub assignment: NodeId,
    /// `const enum`: builds write the value of `Name.Entry` reads in every file of the build instead of reading the
    /// map, which they write all the same.
    pub constant: bool,
}

impl AstNode for Enum {
    fn type_name(&self) -> &'static str {
        "EnumStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "EnumStatement[{}][{}]",
            ctx.location(),
            ctx.child(self.assignment)
        )
    }
}

impl Enum {
    /// `enum Name` with its entries; `constant` after `const`.
    pub(crate) fn parse(parser: &mut Parser<'_>, constant: bool) -> ParseResult<NodeId> {
        let start = parser.token().span();

        parser.advance()?;

        let variable = parser.parse_expr_with(
            None,
            ExprContext {
                as_lval: true,
                statement_start: false,
            },
        )?;

        if !matches!(
            parser.stream.nodes.get(variable).kind,
            NodeKind::Identifier(_)
        ) {
            let span = parser.stream.nodes.get(variable).span;

            parser.report_at(GreybelDiagnostic::EnumName, span)?;
        }

        let mut fields = parser.stream.nodes.list_buffer();
        let mut names = FxHashSet::default();
        let mut next = Some(0.0);

        loop {
            Self::skip_lines(parser)?;

            if parser.token().is_eof() {
                parser.raise_at(GreybelDiagnostic::UnclosedEnum, start)?;

                break;
            }

            if parser.token().is_keyword(Keyword::EndEnum) {
                parser.advance()?;

                break;
            }

            let Some(identifier) = parser.token().as_identifier().copied() else {
                let span = parser.token().span();

                parser.raise_at(GreybelDiagnostic::EnumEntryName, span)?;
                Self::skip_line(parser)?;

                continue;
            };
            let name_span = parser.token().span();
            let name = identifier.name(parser.source());

            if !names.insert(name) {
                parser.report_at(
                    GreybelDiagnostic::DuplicateEnumEntry {
                        name: name.to_owned(),
                    },
                    name_span,
                )?;
            }

            parser.advance()?;

            let value = Self::parse_value(parser, name_span, &mut next)?;
            let key = parser.alloc(
                Literal {
                    value: LiteralValue::String(TextSpan::new(name_span)),
                    negated: false,
                },
                name_span,
            );
            let entry_span = parser.span_from(&name_span);
            let entry = parser.alloc(
                MapEntry {
                    key,
                    value,
                    shorthand: false,
                },
                entry_span,
            );

            fields.push(entry);

            if !(parser.token().is_eol() || parser.token().is_comment() || parser.token().is_eof())
            {
                let span = parser.token().span();

                parser.raise_at(GreybelDiagnostic::EnumEntryName, span)?;
                Self::skip_line(parser)?;
            }
        }

        let span = parser.span_from(&start);
        let fields = parser.stream.nodes.finish_list(fields);
        let map = parser.alloc(Map { fields }, span);
        let assignment = parser.alloc(
            Assignment {
                variable,
                init: map,
                shorthand: false,
                declaration: None,
            },
            span,
        );

        if let Some(scope) = parser
            .stream
            .scopes
            .current()
            .and_then(|scope| parser.stream.scopes.pending_mut(scope))
        {
            scope.definitions.push(assignment);
        }

        Ok(parser.alloc(
            Enum {
                assignment,
                constant,
            },
            span,
        ))
    }

    /// The value after `=`, or the previous number plus 1 (0 first). After a string value the next entry has no
    /// default.
    fn parse_value(
        parser: &mut Parser<'_>,
        name_span: Span,
        next: &mut Option<f64>,
    ) -> ParseResult<NodeId> {
        if !parser.token().is_punct(Punct::Assign) {
            let number = match *next {
                Some(number) => number,
                None => {
                    parser.report_at(GreybelDiagnostic::MissingEnumValue, name_span)?;

                    0.0
                }
            };

            *next = Some(number + 1.0);

            return Ok(parser.alloc(
                Literal {
                    value: LiteralValue::Number(number),
                    negated: false,
                },
                name_span,
            ));
        }

        parser.advance()?;

        let value = parser.parse_expr(None)?;
        let node = parser.stream.nodes.get(value);

        match &node.kind {
            NodeKind::Literal(Literal {
                value: LiteralValue::Number(number),
                negated,
            }) => {
                let number = if *negated { -number } else { *number };

                *next = Some(number + 1.0);
            }
            NodeKind::Literal(Literal {
                value: LiteralValue::String(_),
                ..
            }) => *next = None,
            _ => {
                let span = node.span;

                parser.report_at(GreybelDiagnostic::EnumValue, span)?;
            }
        }

        Ok(value)
    }

    /// Skips line breaks and comments; comments are recorded in the chunk but not attached to a statement, as the
    /// enum is written as it is in dev mode.
    fn skip_lines(parser: &mut Parser<'_>) -> ParseResult<()> {
        loop {
            if parser.token().is_comment() {
                parser.register_comment();
            } else if !parser.token().is_eol() {
                return Ok(());
            }

            parser.advance()?;
        }
    }

    fn skip_line(parser: &mut Parser<'_>) -> ParseResult<()> {
        while !(parser.token().is_eol() || parser.token().is_eof()) {
            parser.advance()?;
        }

        Ok(())
    }
}
