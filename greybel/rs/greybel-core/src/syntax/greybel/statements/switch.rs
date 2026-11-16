use greybel_symbols::contextual_keywords::IS;

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct};
use crate::parser::{Expect, KeywordStatement, Parser, PendingBlock};
use crate::position::Span;
use crate::syntax::{
    Assignment, Binary, BinaryOp, ElseClause, Identifier, IdentifierKind, If, IfClause,
    IfClauseKind, Isa, Logical, LogicalOp, Unary, UnaryOp,
};

/// `switch subject` with `case value, ...` and `else` clauses up to `end switch`. `statements` are the
/// `__s = @subject` and `if __s == value or ... then` chain it stands for; builds write them. `cases` and
/// `fallback` are the clauses as written, for dev mode.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Switch {
    #[child]
    pub statements: NodeList,
    pub subject: NodeId,
    pub cases: Vec<SwitchCase>,
    pub fallback: Option<NodeList>,
}

/// `case "a", "b"` or `case is Dog`, with its body.
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub values: Vec<NodeId>,
    pub type_case: bool,
    pub body: NodeList,
    pub span: Span,
}

impl AstNode for Switch {
    fn type_name(&self) -> &'static str {
        "SwitchStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "SwitchStatement[{}][\n{}\n]",
            ctx.location(),
            ctx.block(self.statements)
        )
    }
}

impl Switch {
    /// Name of the temporary that stores the subject when it isn't a variable.
    pub const SUBJECT: &'static str = "__s";

    /// Parses `switch subject` and opens the block of its clauses.
    pub(crate) fn open(parser: &mut Parser<'_>) -> ParseResult<()> {
        let start = parser.token().span();

        parser.advance()?;

        let subject = parser.parse_expr(None)?;
        let node = parser.alloc(
            Switch {
                statements: NodeList::EMPTY,
                subject,
                cases: Vec::new(),
                fallback: None,
            },
            start.open(),
        );

        parser.attach_pending_comments(node);
        parser.stream.blocks.push(PendingSwitch {
            node,
            subject,
            cases: Vec::new(),
            fallback: None,
            body: Vec::new(),
        });

        Ok(())
    }

    /// `case value, ...` or `case is Type` inside a switch: starts the next clause.
    pub(crate) fn parse_case(parser: &mut Parser<'_>) -> ParseResult<()> {
        let start = parser.token().span();

        parser.advance()?;

        let type_case = parser
            .token()
            .as_identifier()
            .is_some_and(|identifier| identifier.name(parser.source()) == IS);

        if type_case {
            parser.advance()?;
        }

        let mut values = vec![parser.parse_expr(None)?];

        while parser.consume(Expect::Punct(Punct::Comma))? {
            values.push(parser.parse_expr(None)?);
        }

        let span = parser.span_from(&start);

        Self::require_clause_start(parser, start)?;

        if let Some(switch) = parser.stream.blocks.top_mut_as::<PendingSwitch>() {
            switch.start_case(values, type_case, span);
        }

        Ok(())
    }

    /// `else` inside a switch: starts the clause for values no case matches.
    pub(crate) fn parse_fallback(parser: &mut Parser<'_>) -> ParseResult<()> {
        let span = parser.token().span();

        parser.advance()?;
        Self::require_clause_start(parser, span)?;

        let Some(switch) = parser.stream.blocks.top_mut_as::<PendingSwitch>() else {
            return Ok(());
        };

        if switch.fallback.is_some() {
            return parser.raise_at(GreybelDiagnostic::SecondSwitchElse, span);
        }

        switch.start_fallback();

        Ok(())
    }

    /// Reports statements of a switch before its first `case`, which no clause would run.
    fn require_clause_start(parser: &mut Parser<'_>, span: Span) -> ParseResult<()> {
        let outside = parser
            .stream
            .blocks
            .top_mut_as::<PendingSwitch>()
            .is_some_and(|switch| {
                switch.cases.is_empty() && switch.fallback.is_none() && !switch.body.is_empty()
            });

        if outside {
            return parser.raise_at(GreybelDiagnostic::StatementOutsideCase, span);
        }

        Ok(())
    }
}

struct PendingSwitch {
    node: NodeId,
    subject: NodeId,
    cases: Vec<(Vec<NodeId>, bool, Vec<NodeId>, Span)>,
    fallback: Option<Vec<NodeId>>,
    /// The statements of the open clause.
    body: Vec<NodeId>,
}

impl PendingSwitch {
    fn close_clause(&mut self) {
        let body = std::mem::take(&mut self.body);

        if let Some(fallback) = &mut self.fallback {
            fallback.extend(body);
        } else if let Some(case) = self.cases.last_mut() {
            case.2.extend(body);
        }
    }

    fn start_case(&mut self, values: Vec<NodeId>, type_case: bool, span: Span) {
        self.close_clause();
        self.cases.push((values, type_case, Vec::new(), span));
    }

    fn start_fallback(&mut self) {
        self.close_clause();
        self.fallback = Some(Vec::new());
    }
}

impl PendingBlock for PendingSwitch {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.body
    }

    fn name(&self) -> &'static str {
        "switch"
    }

    fn start(&self, nodes: &NodeStore) -> u32 {
        nodes.get(self.node).span.start
    }

    fn complete(mut self: Box<Self>, parser: &mut Parser<'_>, end: &Span) {
        self.close_clause();

        let origin = parser.stream.nodes.get(self.node).span.start;
        let at = Span::empty(origin);
        let mut statements = Vec::new();
        let subject = if matches!(
            parser.stream.nodes.get(self.subject).kind,
            NodeKind::Identifier(_)
        ) {
            self.subject
        } else {
            let temporary = Self::temporary(parser, at);
            let span = parser.stream.nodes.get(self.subject).span;
            let assignment = parser.alloc(
                Assignment {
                    variable: temporary,
                    init: self.subject,
                    shorthand: false,
                    declaration: None,
                },
                span,
            );

            parser.scope_data_mut().definitions.push(assignment);
            statements.push(assignment);

            temporary
        };
        let mut clauses = Vec::new();
        let mut cases = Vec::new();

        for (index, (values, type_case, body, span)) in self.cases.iter().enumerate() {
            let condition = Self::condition(parser, subject, values, *type_case, *span);
            let body = parser.stream.nodes.push_list(body);
            let kind = if index == 0 {
                IfClauseKind::If
            } else {
                IfClauseKind::ElseIf
            };

            clauses.push(parser.alloc(
                IfClause {
                    kind,
                    shortcut: false,
                    condition,
                    body,
                },
                *span,
            ));
            cases.push(SwitchCase {
                values: values.clone(),
                type_case: *type_case,
                body,
                span: *span,
            });
        }

        let fallback = self
            .fallback
            .as_ref()
            .map(|body| parser.stream.nodes.push_list(body));

        if let Some(body) = fallback {
            clauses.push(parser.alloc(
                ElseClause {
                    shortcut: false,
                    body,
                },
                at,
            ));
        }

        if !clauses.is_empty() {
            let clauses = parser.stream.nodes.push_list(&clauses);

            statements.push(parser.alloc(
                If {
                    shortcut: false,
                    clauses,
                },
                at,
            ));
        }

        let statements = parser.stream.nodes.push_list(&statements);

        parser.stream.nodes.close(self.node, end);
        parser.stream.nodes.set_kind(
            self.node,
            Switch {
                statements,
                subject: self.subject,
                cases,
                fallback,
            },
        );
        parser.register_line_item(self.node);
    }
}

impl PendingSwitch {
    fn temporary(parser: &mut Parser<'_>, at: Span) -> NodeId {
        let name = parser.stream.nodes.insert_atom(Switch::SUBJECT);
        let identifier = parser.alloc(
            Identifier {
                name,
                kind: IdentifierKind::Variable,
            },
            at,
        );

        parser.scope_data_mut().namespaces.push(identifier);

        identifier
    }

    /// `subject == a or subject == b`, or `subject isa Type` for a type case.
    fn condition(
        parser: &mut Parser<'_>,
        subject: NodeId,
        values: &[NodeId],
        type_case: bool,
        span: Span,
    ) -> NodeId {
        let mut condition: Option<NodeId> = None;

        for &value in values {
            let left = parser.stream.nodes.clone_subtree(subject);
            let left = if type_case {
                left
            } else {
                parser.alloc(
                    Unary {
                        operator: UnaryOp::AddressOf,
                        argument: left,
                    },
                    span,
                )
            };
            let test = if type_case {
                parser.alloc(Isa { left, right: value }, span)
            } else {
                parser.alloc(
                    Binary {
                        operator: BinaryOp::Equal,
                        left,
                        right: value,
                    },
                    span,
                )
            };

            condition = Some(match condition {
                Some(previous) => parser.alloc(
                    Logical {
                        operator: LogicalOp::Or,
                        left: previous,
                        right: test,
                    },
                    span,
                ),
                None => test,
            });
        }

        condition.expect("a case has a value")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndSwitchRule;

impl KeywordStatement for EndSwitchRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::EndSwitch]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.parse_block_end(|parser| {
            let Some(node) = parser
                .stream
                .blocks
                .top_mut_as::<PendingSwitch>()
                .map(|block| block.node)
            else {
                return parser.raise_without_starter(Keyword::EndSwitch.as_str());
            };
            let end = parser.prev().span();

            parser.complete_top_block(&end);
            parser.stream.blocks.push_statement(node);

            Ok(())
        })
    }
}

impl Parser<'_> {
    /// Whether the open block is a `switch`, where `case` and `else` start clauses.
    #[must_use]
    pub(crate) fn is_in_switch(&self) -> bool {
        self.stream.blocks.top_is::<PendingSwitch>()
    }
}
