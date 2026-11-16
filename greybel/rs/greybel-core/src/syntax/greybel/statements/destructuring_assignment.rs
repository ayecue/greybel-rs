use greybel_symbols::contextual_keywords::AS;

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList, TextSpan};
use crate::error::ParseResult;
use crate::lexer::Punct;
use crate::parser::{Expect, Parser};
use crate::position::Span;
use crate::syntax::{
    Assignment, Binary, BinaryOp, Call, ElseClause, Empty, Identifier, IdentifierKind, If,
    IfClause, IfClauseKind, Index, Literal, LiteralValue, Logical, LogicalOp, Member, Slice, Unary,
    UnaryOp,
};

/// `[user, host] = expr` or `{name, port as p, timeout = 30} = expr`. `statements` are the assignments it stands
/// for (`__d0 = expr`, `user = @__d0[0]`, ...); builds write them.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct DestructuringAssignment {
    #[child]
    pub statements: NodeList,
}

impl AstNode for DestructuringAssignment {
    fn type_name(&self) -> &'static str {
        "DestructuringAssignmentStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "DestructuringAssignmentStatement[{}][\n{}\n]",
            ctx.location(),
            ctx.block(self.statements)
        )
    }
}

pub(crate) enum DestructuringTarget {
    List(Vec<ListElement>),
    Map(Vec<MapElement>),
}

pub(crate) enum Target {
    Name(NodeId),
    Inner(DestructuringTarget),
}

pub(crate) struct ListElement {
    target: Target,
    default: Option<NodeId>,
    remainder: bool,
    span: Span,
}

pub(crate) struct MapElement {
    key: NodeId,
    target: Target,
    default: Option<NodeId>,
    span: Span,
}

impl DestructuringAssignment {
    /// Prefix of the temporaries that store the destructured values, followed by the nesting depth.
    pub const TEMPORARY_PREFIX: &'static str = "__d";

    /// Whether the statement at `start` is `[...]` or `{...}` followed by `=` (not `==`) on the same line.
    #[must_use]
    pub fn is_at(source: &str, start: u32) -> bool {
        let bytes = source.as_bytes();
        let mut depth = 0_usize;
        let mut index = start as usize;

        while index < bytes.len() {
            match bytes[index] {
                b'[' | b'{' | b'(' => depth += 1,
                b']' | b'}' | b')' => {
                    depth = depth.saturating_sub(1);

                    if depth == 0 {
                        let after = source[index + 1..].trim_start_matches([' ', '\t']);

                        return after.starts_with('=') && !after.starts_with("==");
                    }
                }
                b'"' => {
                    index += 1;

                    while index < bytes.len() && bytes[index] != b'"' {
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

    pub(crate) fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let start = parser.token().span();
        let target = Self::parse_target(parser)?;

        parser.require(Expect::Punct(Punct::Assign), Some(start.start))?;

        let value = parser.parse_expr(None)?;
        let span = parser.span_from(&start);
        let statements = DestructuringExpansion::expand(parser, &target, value, span.start)?;
        let statements = parser.stream.nodes.push_list(&statements);

        Ok(parser.alloc(DestructuringAssignment { statements }, span))
    }

    pub(crate) fn parse_target(parser: &mut Parser<'_>) -> ParseResult<DestructuringTarget> {
        if parser.token().is_punct(Punct::LBrace) {
            return Self::parse_map_target(parser);
        }

        let start = parser.token().span();
        let mut elements = Vec::new();

        parser.require(Expect::Punct(Punct::LBracket), Some(start.start))?;

        while !parser.token().is_punct(Punct::RBracket) && !parser.token().is_eol() {
            let element_start = parser.token().span();
            let remainder = parser.consume(Expect::Punct(Punct::Ellipsis))?;
            let target = Self::parse_element_target(parser)?;
            let default = Self::parse_default(parser)?;

            elements.push(ListElement {
                target,
                default,
                remainder,
                span: parser.span_from(&element_start),
            });

            if !parser.consume(Expect::Punct(Punct::Comma))? {
                break;
            }
        }

        parser.require(Expect::Punct(Punct::RBracket), Some(start.start))?;

        Ok(DestructuringTarget::List(elements))
    }

    fn parse_map_target(parser: &mut Parser<'_>) -> ParseResult<DestructuringTarget> {
        let start = parser.token().span();
        let mut elements = Vec::new();

        parser.require(Expect::Punct(Punct::LBrace), Some(start.start))?;

        while !parser.token().is_punct(Punct::RBrace) && !parser.token().is_eol() {
            let element_start = parser.token().span();
            let renamed =
                parser.token().as_identifier().is_some() && Self::is_word_after(parser, AS);
            let (key, target) = if renamed {
                let key = Identifier::parse(parser, IdentifierKind::Property)?;

                parser.advance()?;

                (key, Self::parse_element_target(parser)?)
            } else {
                let target = Identifier::parse(parser, IdentifierKind::Variable)?;
                let key = parser.stream.nodes.clone_subtree(target);

                if let NodeKind::Identifier(identifier) = &mut parser.stream.nodes.get_mut(key).kind
                {
                    identifier.kind = IdentifierKind::Property;
                }

                (key, Target::Name(target))
            };
            let default = Self::parse_default(parser)?;

            elements.push(MapElement {
                key,
                target,
                default,
                span: parser.span_from(&element_start),
            });

            if !parser.consume(Expect::Punct(Punct::Comma))? {
                break;
            }
        }

        parser.require(Expect::Punct(Punct::RBrace), Some(start.start))?;

        Ok(DestructuringTarget::Map(elements))
    }

    fn parse_element_target(parser: &mut Parser<'_>) -> ParseResult<Target> {
        if parser.token().is_punct(Punct::LBracket) || parser.token().is_punct(Punct::LBrace) {
            return Ok(Target::Inner(Self::parse_target(parser)?));
        }

        Ok(Target::Name(Identifier::parse(
            parser,
            IdentifierKind::Variable,
        )?))
    }

    fn parse_default(parser: &mut Parser<'_>) -> ParseResult<Option<NodeId>> {
        if !parser.consume(Expect::Punct(Punct::Assign))? {
            return Ok(None);
        }

        Ok(Some(parser.parse_expr(None)?))
    }

    /// Whether the word after the current token is `word`.
    fn is_word_after(parser: &Parser<'_>, word: &str) -> bool {
        let after = &parser.source()[parser.token().span().end as usize..];
        let after = after.trim_start_matches([' ', '\t']);

        after.strip_prefix(word).is_some_and(|remainder| {
            remainder.starts_with([' ', '\t']) || remainder.starts_with(['[', '{'])
        })
    }
}

/// Builds the statements a destructuring target stands for.
pub(crate) struct DestructuringExpansion<'p, 'src> {
    parser: &'p mut Parser<'src>,
    statements: Vec<NodeId>,
    depth: usize,
    origin: u32,
}

impl<'p, 'src> DestructuringExpansion<'p, 'src> {
    /// The statements that assign the parts of `value` to the names of `target`; synthesized nodes get an empty
    /// span at `origin`.
    pub(crate) fn expand(
        parser: &'p mut Parser<'src>,
        target: &DestructuringTarget,
        value: NodeId,
        origin: u32,
    ) -> ParseResult<Vec<NodeId>> {
        let mut expansion = Self {
            parser,
            statements: Vec::new(),
            depth: 0,
            origin,
        };

        expansion.lower(target, value)?;

        Ok(expansion.statements)
    }

    fn lower(&mut self, target: &DestructuringTarget, value: NodeId) -> ParseResult<()> {
        let base = if matches!(
            self.parser.stream.nodes.get(value).kind,
            NodeKind::Identifier(_)
        ) {
            value
        } else {
            let temporary = self.temporary();
            let span = self.parser.stream.nodes.get(value).span;
            let assignment = self.assignment(temporary, value, span);

            self.statements.push(assignment);

            temporary
        };

        match target {
            DestructuringTarget::List(elements) => {
                for (position, element) in elements.iter().enumerate() {
                    let read = self.list_read(base, position, element);
                    let present = element.default.map(|_| {
                        let base = self.copy(base);
                        let length = self.member(base, "len");
                        let index = self.number(position);

                        self.parser.alloc(
                            Binary {
                                operator: BinaryOp::Greater,
                                left: length,
                                right: index,
                            },
                            element.span,
                        )
                    });

                    self.bind(
                        &element.target,
                        read,
                        element.default,
                        present,
                        element.span,
                    )?;
                }
            }
            DestructuringTarget::Map(elements) => {
                for element in elements {
                    let base_copy = self.copy(base);
                    let key = self.parser.stream.nodes.clone_subtree(element.key);
                    let read = self.parser.alloc(
                        Member {
                            base: base_copy,
                            identifier: key,
                            optional: false,
                        },
                        element.span,
                    );
                    let read = self.reference(read, element.span);
                    let present = element.default.map(|_| {
                        let base = self.copy(base);
                        let has_index = self.member(base, "hasIndex");
                        let key_span = self.parser.stream.nodes.get(element.key).span;
                        let name = self.parser.alloc(
                            Literal {
                                value: LiteralValue::String(TextSpan::new(key_span)),
                                negated: false,
                            },
                            key_span,
                        );
                        let arguments = self.parser.stream.nodes.push_list(&[name]);

                        self.parser.alloc(
                            Call {
                                base: has_index,
                                arguments,
                                optional: false,
                            },
                            element.span,
                        )
                    });

                    self.bind(
                        &element.target,
                        read,
                        element.default,
                        present,
                        element.span,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Assigns `read` to `target`; with a default, `if present and read != null then target = read else
    /// target = default`.
    fn bind(
        &mut self,
        target: &Target,
        read: NodeId,
        default: Option<NodeId>,
        present: Option<NodeId>,
        span: Span,
    ) -> ParseResult<()> {
        let name = match target {
            Target::Name(name) => *name,
            Target::Inner(target) => {
                self.depth += 1;
                self.lower(target, read)?;
                self.depth -= 1;

                return Ok(());
            }
        };

        let (Some(default), Some(present)) = (default, present) else {
            let assignment = self.assignment(name, read, span);

            self.statements.push(assignment);

            return Ok(());
        };

        let read_copy = self.parser.stream.nodes.clone_subtree(read);
        let null = self.parser.alloc(
            Literal {
                value: LiteralValue::Nil,
                negated: false,
            },
            self.at(),
        );
        let not_null = self.parser.alloc(
            Binary {
                operator: BinaryOp::NotEqual,
                left: read_copy,
                right: null,
            },
            span,
        );
        let condition = self.parser.alloc(
            Logical {
                operator: LogicalOp::And,
                left: present,
                right: not_null,
            },
            span,
        );
        let fallback_name = self.parser.stream.nodes.clone_subtree(name);
        let assign_read = self.assignment(name, read, span);
        let assign_default = self.assignment(fallback_name, default, span);
        let then_body = self.parser.stream.nodes.push_list(&[assign_read]);
        let else_body = self.parser.stream.nodes.push_list(&[assign_default]);
        let clause = self.parser.alloc(
            IfClause {
                kind: IfClauseKind::If,
                shortcut: true,
                condition,
                body: then_body,
            },
            span,
        );
        let fallback = self.parser.alloc(
            ElseClause {
                shortcut: true,
                body: else_body,
            },
            span,
        );
        let clauses = self.parser.stream.nodes.push_list(&[clause, fallback]);
        let statement = self.parser.alloc(
            If {
                shortcut: true,
                clauses,
            },
            span,
        );

        self.statements.push(statement);

        Ok(())
    }

    fn assignment(&mut self, variable: NodeId, init: NodeId, span: Span) -> NodeId {
        let assignment = self.parser.alloc(
            Assignment {
                variable,
                init,
                shorthand: false,
                declaration: None,
            },
            span,
        );

        self.parser.scope_data_mut().definitions.push(assignment);

        assignment
    }

    /// `@read`, so a function value isn't called.
    fn reference(&mut self, read: NodeId, span: Span) -> NodeId {
        self.parser.alloc(
            Unary {
                operator: UnaryOp::AddressOf,
                argument: read,
            },
            span,
        )
    }

    fn temporary(&mut self) -> NodeId {
        let text = format!(
            "{}{}",
            DestructuringAssignment::TEMPORARY_PREFIX,
            self.depth
        );
        let name = self.parser.stream.nodes.insert_atom(&text);
        let at = self.at();
        let identifier = self.parser.alloc(
            Identifier {
                name,
                kind: IdentifierKind::Variable,
            },
            at,
        );

        self.parser.scope_data_mut().namespaces.push(identifier);

        identifier
    }

    /// `@base[position]`, or `base[position:]` for `...name`.
    fn list_read(&mut self, base: NodeId, position: usize, element: &ListElement) -> NodeId {
        let base = self.copy(base);
        let left = self.number(position);

        if element.remainder {
            let right = self.parser.alloc(Empty, self.at());

            return self.parser.alloc(
                Slice {
                    base,
                    left,
                    right,
                    optional: false,
                },
                element.span,
            );
        }

        let read = self.parser.alloc(
            Index {
                base,
                index: left,
                is_statement_start: false,
                optional: false,
            },
            element.span,
        );

        self.reference(read, element.span)
    }

    fn copy(&mut self, node: NodeId) -> NodeId {
        self.parser.stream.nodes.clone_subtree(node)
    }

    fn member(&mut self, base: NodeId, name: &str) -> NodeId {
        let name = self.parser.stream.nodes.insert_atom(name);
        let at = self.at();
        let identifier = self.parser.alloc(
            Identifier {
                name,
                kind: IdentifierKind::Property,
            },
            at,
        );

        self.parser.alloc(
            Member {
                base,
                identifier,
                optional: false,
            },
            at,
        )
    }

    fn number(&mut self, value: usize) -> NodeId {
        let at = self.at();

        self.parser.alloc(
            Literal {
                value: LiteralValue::Number(f64::from(u32::try_from(value).unwrap_or(u32::MAX))),
                negated: false,
            },
            at,
        )
    }

    /// The empty span synthesized nodes get, at the statement start.
    fn at(&self) -> Span {
        Span::empty(self.origin)
    }
}
