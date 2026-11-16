use greybel_symbols::contextual_keywords::{
    CONSTRUCTOR, CREATE, EXTENDS, IMPLEMENTS, PRIVATE, STATIC,
};
use greybel_symbols::names::SELF;
use rustc_hash::FxHashSet;

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, NodeKind, NodeList, NodeStore};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct, is_identifier_start};
use crate::parser::{Expect, ExprContext, KeywordStatement, Parser, PendingBlock};
use crate::position::Span;
use crate::syntax::{
    Assignment, Call, Function, Identifier, IdentifierKind, Invalid, Map, Member, Parenthesis,
    Return, Unary, UnaryOp, Unknown,
};

#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct Class {
    #[child]
    pub statements: NodeList,
    pub name: NodeId,
    pub base: Option<NodeId>,
    pub implements: Vec<Span>,
    pub members: Vec<ClassMember>,
    pub body: NodeList,
    pub constructor: Option<WrittenConstructor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassMember {
    pub assignment: NodeId,
    pub private: bool,
    pub is_static: bool,
    pub method: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenConstructor {
    pub function: NodeId,
    pub body: NodeList,
    pub self_returns: Vec<NodeId>,
}

impl AstNode for Class {
    fn type_name(&self) -> &'static str {
        "ClassStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        format!(
            "ClassStatement[{}][\n{}\n]",
            ctx.location(),
            ctx.block(self.statements)
        )
    }
}

impl Class {
    pub(crate) fn open(parser: &mut Parser<'_>) -> ParseResult<()> {
        let start = parser.token().span();

        parser.advance()?;

        let name = parser.parse_expr_with(
            None,
            ExprContext {
                as_lval: true,
                statement_start: false,
            },
        )?;

        if !matches!(parser.stream.nodes.get(name).kind, NodeKind::Identifier(_)) {
            let span = parser.stream.nodes.get(name).span;

            parser.report_at(GreybelDiagnostic::ClassName, span)?;
        }

        let base = if Self::is_at_word(parser, EXTENDS) {
            parser.advance()?;

            Some(parser.parse_expr(None)?)
        } else {
            None
        };
        let mut implements = Vec::new();

        if Self::is_at_word(parser, IMPLEMENTS) {
            parser.advance()?;

            loop {
                if parser.token().as_identifier().is_none() {
                    let span = parser.token().span();

                    parser.raise_at(GreybelDiagnostic::ImplementsName, span)?;

                    break;
                }

                implements.push(parser.token().span());
                parser.advance()?;

                if !parser.consume(Expect::Punct(Punct::Comma))? {
                    break;
                }
            }
        }

        let header = parser.span_from(&start);
        let node = parser.alloc(
            Class {
                statements: NodeList::EMPTY,
                name,
                base,
                implements: implements.clone(),
                members: Vec::new(),
                body: NodeList::EMPTY,
                constructor: None,
            },
            start.open(),
        );

        parser.attach_pending_comments(node);
        parser.stream.blocks.push(PendingClass {
            node,
            header,
            name,
            base,
            implements,
            names: FxHashSet::default(),
            private: FxHashSet::default(),
            statics: FxHashSet::default(),
            methods: FxHashSet::default(),
            body: Vec::new(),
        });

        Ok(())
    }

    pub(crate) fn parse_member(parser: &mut Parser<'_>) -> ParseResult<()> {
        let (mut private, mut is_static) = (false, false);

        loop {
            if !private && Self::is_at_word(parser, PRIVATE) && Self::is_followed_by_name(parser) {
                private = true;
            } else if !is_static
                && Self::is_at_word(parser, STATIC)
                && Self::is_followed_by_name(parser)
            {
                is_static = true;
            } else {
                break;
            }

            parser.advance()?;
        }

        let start = parser.token().span();
        let Some(identifier) = parser.token().as_identifier().copied() else {
            return Self::reject_line(parser, start);
        };
        let name = identifier.name(parser.source()).to_owned();
        let after = parser.source()[start.end as usize..].trim_start_matches([' ', '\t']);
        let method = after.starts_with('(');
        let assigns = after.starts_with('=') && !after.starts_with("==");

        if !method && !assigns {
            return Self::reject_line(parser, start);
        }

        let Some(class) = parser.stream.blocks.top_mut_as::<PendingClass>() else {
            return Ok(());
        };
        let is_new = class.names.insert(name.clone());

        if private {
            class.private.insert(name.clone());
        }

        if is_static {
            class.statics.insert(name.clone());
        }

        if method {
            class.methods.insert(name.clone());
        }

        if !is_new {
            parser.report_at(GreybelDiagnostic::DuplicateClassMember { name }, start)?;
        }

        let depth = parser.stream.blocks.depth();
        let variable = Identifier::parse(parser, IdentifierKind::Property)?;

        if method {
            let assignment = parser.alloc(Invalid, start.open());
            let function = Function::parse_method(parser, assignment)?;

            parser.stream.nodes.set_kind(
                assignment,
                Assignment {
                    variable,
                    init: function,
                    shorthand: false,
                    declaration: None,
                },
            );
            parser.emit_statement(depth, assignment, false);
        } else {
            let assignment = Assignment::parse(parser, variable, start)?;
            let closed = parser.stream.nodes.get(assignment).span.is_closed();

            parser.emit_statement(depth, assignment, closed);
        }

        Ok(())
    }

    fn reject_line(parser: &mut Parser<'_>, span: Span) -> ParseResult<()> {
        parser.raise_at(GreybelDiagnostic::ClassMember, span)?;

        while !(parser.token().is_eol() || parser.token().is_eof() || parser.token().is_comment()) {
            parser.advance()?;
        }

        Ok(())
    }

    fn is_at_word(parser: &Parser<'_>, word: &str) -> bool {
        parser
            .token()
            .as_identifier()
            .is_some_and(|identifier| identifier.name(parser.source()) == word)
    }

    fn is_followed_by_name(parser: &Parser<'_>) -> bool {
        let span = parser.token().span();
        let after = &parser.source().as_bytes()[span.end as usize..];
        let spaces = after
            .iter()
            .take_while(|&&byte| byte == b' ' || byte == b'\t')
            .count();

        spaces > 0
            && after
                .get(spaces)
                .is_some_and(|&byte| is_identifier_start(byte))
    }
}

struct PendingClass {
    node: NodeId,
    header: Span,
    name: NodeId,
    base: Option<NodeId>,
    implements: Vec<Span>,
    names: FxHashSet<String>,
    private: FxHashSet<String>,
    statics: FxHashSet<String>,
    methods: FxHashSet<String>,
    body: Vec<NodeId>,
}

impl PendingClass {
    fn member_name(nodes: &NodeStore, assignment: NodeId) -> Option<&str> {
        let NodeKind::Assignment(assignment) = &nodes.get(assignment).kind else {
            return None;
        };

        match &nodes.get(assignment.variable).kind {
            NodeKind::Identifier(identifier) => Some(nodes.resolve(identifier.name)),
            _ => None,
        }
    }

    fn is_instance_field(&self, name: &str) -> bool {
        !self.statics.contains(name) && !self.methods.contains(name)
    }

    fn requires_constructor(&self, nodes: &NodeStore) -> bool {
        if self.base.is_none() || self.names.contains(CONSTRUCTOR) {
            return false;
        }

        self.body.iter().any(|&assignment| {
            Self::member_name(nodes, assignment).is_some_and(|name| self.is_instance_field(name))
        })
    }

    fn identifier(parser: &mut Parser<'_>, name: &str, kind: IdentifierKind, at: u32) -> NodeId {
        let name = parser.stream.nodes.insert_atom(name);
        let identifier = parser.alloc(Identifier { name, kind }, Span::empty(at));

        if kind != IdentifierKind::Property {
            parser.scope_data_mut().namespaces.push(identifier);
        }

        identifier
    }

    fn instance_assignments(parser: &mut Parser<'_>, fields: &[NodeId], at: u32) -> Vec<NodeId> {
        for &assignment in fields {
            let NodeKind::Assignment(node) = parser.stream.nodes.get(assignment).kind.clone()
            else {
                continue;
            };
            let span = parser.stream.nodes.get(node.variable).span;
            let receiver = Self::identifier(parser, SELF, IdentifierKind::Variable, at);
            let target = parser.alloc(
                Member {
                    base: receiver,
                    identifier: node.variable,
                    optional: false,
                },
                span,
            );

            parser.stream.nodes.set_kind(
                assignment,
                Assignment {
                    variable: target,
                    ..node
                },
            );
            parser.scope_data_mut().definitions.push(assignment);
        }

        fields.to_vec()
    }

    fn return_self(parser: &mut Parser<'_>, at: u32) -> NodeId {
        let receiver = Self::identifier(parser, SELF, IdentifierKind::Variable, at);
        let statement = parser.alloc(
            Return {
                argument: Some(receiver),
            },
            Span::empty(at),
        );

        parser.scope_data_mut().returns.push(statement);

        statement
    }

    fn extend_constructor(
        parser: &mut Parser<'_>,
        function: NodeId,
        fields: &[NodeId],
    ) -> WrittenConstructor {
        let NodeKind::Function(node) = parser.stream.nodes.get(function).kind.clone() else {
            unreachable!("the constructor is a function");
        };
        let start = parser.stream.nodes.get(function).span.start;
        let end = parser.stream.nodes.get(function).span.end;

        parser.resume_scope(function);

        let mut body = Self::instance_assignments(parser, fields, start);
        let returns = parser.scope_data_mut().returns.clone();
        let mut self_returns = Vec::new();

        for statement in returns {
            if let NodeKind::Return(Return { argument: None }) =
                parser.stream.nodes.get(statement).kind
            {
                self_returns.push(statement);

                let at = parser.stream.nodes.get(statement).span.end;
                let receiver = Self::identifier(parser, SELF, IdentifierKind::Variable, at);

                parser.stream.nodes.set_kind(
                    statement,
                    Return {
                        argument: Some(receiver),
                    },
                );
            }
        }

        let statements = parser.stream.nodes.list(node.body).to_vec();
        let ends_with_return = statements
            .last()
            .is_some_and(|&last| matches!(parser.stream.nodes.get(last).kind, NodeKind::Return(_)));

        body.extend(statements);

        if !ends_with_return {
            body.push(Self::return_self(parser, end));
        }

        parser.exit_scope();

        let body = parser.stream.nodes.push_list(&body);

        if let NodeKind::Function(node) = &mut parser.stream.nodes.get_mut(function).kind {
            node.body = body;
        }

        WrittenConstructor {
            function,
            body: node.body,
            self_returns,
        }
    }

    fn generated_constructor(&self, parser: &mut Parser<'_>, fields: &[NodeId], at: u32) -> NodeId {
        let assignment = parser.alloc(Invalid, Span::empty(at));
        let function = parser.alloc(
            Function {
                assignment: Some(assignment),
                ..Function::default()
            },
            Span::empty(at),
        );

        parser.enter_scope(function);

        let mut body = Self::instance_assignments(parser, fields, at);

        body.push(Self::return_self(parser, at));
        parser.exit_scope();

        let body = parser.stream.nodes.push_list(&body);

        if let NodeKind::Function(node) = &mut parser.stream.nodes.get_mut(function).kind {
            node.body = body;
        }

        let owner = parser.stream.nodes.clone_subtree(self.name);
        let name_start = parser.stream.nodes.get(self.name).span.start;
        let identifier =
            Self::identifier(parser, CONSTRUCTOR, IdentifierKind::Property, name_start);
        let variable = parser.alloc(
            Member {
                base: owner,
                identifier,
                optional: false,
            },
            Span::empty(at),
        );

        parser.stream.nodes.set_kind(
            assignment,
            Assignment {
                variable,
                init: function,
                shorthand: false,
                declaration: None,
            },
        );
        parser.scope_data_mut().definitions.push(assignment);

        assignment
    }
}

impl PendingClass {
    fn create_parameters(
        parser: &mut Parser<'_>,
        constructor: Option<NodeId>,
    ) -> (Vec<NodeId>, Vec<NodeId>) {
        let constructor_parameters = constructor
            .map(
                |constructor| match &parser.stream.nodes.get(constructor).kind {
                    NodeKind::Function(node) => parser.stream.nodes.list(node.parameters).to_vec(),
                    _ => Vec::new(),
                },
            )
            .unwrap_or_default();
        let mut parameters = Vec::new();
        let mut arguments = Vec::new();

        for parameter in constructor_parameters {
            let copy = parser.stream.nodes.clone_subtree(parameter);
            let default = match &parser.stream.nodes.get(copy).kind {
                NodeKind::Assignment(node) => Some(node.variable),
                _ => None,
            };
            let identifier = match default {
                Some(variable) => {
                    parser.scope_data_mut().definitions.push(copy);

                    variable
                }
                _ => {
                    let span = parser.stream.nodes.get(copy).span;
                    let unknown = parser.alloc(Unknown, span);
                    let definition = parser.alloc(
                        Assignment {
                            variable: copy,
                            init: unknown,
                            shorthand: false,
                            declaration: None,
                        },
                        span,
                    );

                    parser.scope_data_mut().definitions.push(definition);

                    copy
                }
            };
            let NodeKind::Identifier(name) = parser.stream.nodes.get(identifier).kind.clone()
            else {
                continue;
            };
            let span = parser.stream.nodes.get(identifier).span;

            parser.scope_data_mut().namespaces.push(identifier);

            let read = parser.alloc(
                Identifier {
                    name: name.name,
                    kind: IdentifierKind::Variable,
                },
                span,
            );

            parser.scope_data_mut().namespaces.push(read);
            arguments.push(parser.alloc(
                Unary {
                    operator: UnaryOp::AddressOf,
                    argument: read,
                },
                span,
            ));
            parameters.push(copy);
        }

        (parameters, arguments)
    }

    fn create(&self, parser: &mut Parser<'_>, constructor: Option<NodeId>, at: u32) -> NodeId {
        let assignment = parser.alloc(Invalid, Span::empty(at));
        let function = parser.alloc(
            Function {
                assignment: Some(assignment),
                ..Function::default()
            },
            Span::empty(at),
        );

        parser.enter_scope(function);

        let (parameters, arguments) = Self::create_parameters(parser, constructor);
        let receiver = Self::identifier(parser, SELF, IdentifierKind::Variable, at);
        let instance = parser.alloc(
            Unary {
                operator: UnaryOp::New,
                argument: receiver,
            },
            Span::empty(at),
        );
        let value = if constructor.is_some() {
            let base = parser.alloc(
                Parenthesis {
                    expression: instance,
                },
                Span::empty(at),
            );
            let identifier = Self::identifier(parser, CONSTRUCTOR, IdentifierKind::Property, at);
            let method = parser.alloc(
                Member {
                    base,
                    identifier,
                    optional: false,
                },
                Span::empty(at),
            );
            let arguments = parser.stream.nodes.push_list(&arguments);

            parser.alloc(
                Call {
                    base: method,
                    arguments,
                    optional: false,
                },
                Span::empty(at),
            )
        } else {
            instance
        };
        let statement = parser.alloc(
            Return {
                argument: Some(value),
            },
            Span::empty(at),
        );

        parser.scope_data_mut().returns.push(statement);
        parser.exit_scope();

        let parameters = parser.stream.nodes.push_list(&parameters);
        let body = parser.stream.nodes.push_list(&[statement]);

        if let NodeKind::Function(node) = &mut parser.stream.nodes.get_mut(function).kind {
            node.parameters = parameters;
            node.body = body;
        }

        let owner = parser.stream.nodes.clone_subtree(self.name);
        let name_start = parser.stream.nodes.get(self.name).span.start;
        let identifier = Self::identifier(parser, CREATE, IdentifierKind::Property, name_start);
        let variable = parser.alloc(
            Member {
                base: owner,
                identifier,
                optional: false,
            },
            Span::empty(at),
        );

        parser.stream.nodes.set_kind(
            assignment,
            Assignment {
                variable,
                init: function,
                shorthand: false,
                declaration: None,
            },
        );
        parser.scope_data_mut().definitions.push(assignment);

        assignment
    }
}

impl PendingClass {
    fn head(&self, parser: &mut Parser<'_>) -> NodeId {
        let header = self.header;
        let init = match self.base {
            Some(argument) => {
                let span = parser.stream.nodes.get(argument).span;

                parser.alloc(
                    Unary {
                        operator: UnaryOp::New,
                        argument,
                    },
                    span,
                )
            }
            None => parser.alloc(
                Map {
                    fields: NodeList::EMPTY,
                },
                Span::empty(header.start),
            ),
        };
        let head = parser.alloc(
            Assignment {
                variable: self.name,
                init,
                shorthand: false,
                declaration: None,
            },
            header,
        );

        parser.scope_data_mut().definitions.push(head);

        head
    }
}

impl PendingBlock for PendingClass {
    fn body_mut(&mut self) -> &mut Vec<NodeId> {
        &mut self.body
    }

    fn name(&self) -> &'static str {
        "class"
    }

    fn start(&self, nodes: &NodeStore) -> u32 {
        nodes.get(self.node).span.start
    }

    fn complete(self: Box<Self>, parser: &mut Parser<'_>, end: &Span) {
        let head = self.head(parser);
        let mut statements = vec![head];
        let mut members = Vec::new();
        let mut fields = Vec::new();
        let mut constructor = None;

        for &assignment in &self.body {
            let NodeKind::Assignment(node) = parser.stream.nodes.get(assignment).kind.clone()
            else {
                continue;
            };
            let name = Self::member_name(&parser.stream.nodes, assignment)
                .unwrap_or_default()
                .to_owned();
            let member = ClassMember {
                assignment,
                private: self.private.contains(&name),
                is_static: self.statics.contains(&name),
                method: self.methods.contains(&name),
            };

            members.push(member);

            if self.is_instance_field(&name) {
                fields.push(assignment);

                continue;
            }

            let at = parser.stream.nodes.get(node.variable).span;
            let owner = parser.stream.nodes.clone_subtree(self.name);
            let target = parser.alloc(
                Member {
                    base: owner,
                    identifier: node.variable,
                    optional: false,
                },
                at,
            );

            parser.stream.nodes.set_kind(
                assignment,
                Assignment {
                    variable: target,
                    ..node
                },
            );

            let is_function = matches!(
                parser.stream.nodes.get(node.init).kind,
                NodeKind::Function(_)
            );

            if name == CONSTRUCTOR && is_function {
                constructor = Some(node.init);
            }

            statements.push(assignment);
        }

        let written_constructor = match constructor {
            Some(function) => Some(Self::extend_constructor(parser, function, &fields)),
            None if !fields.is_empty() => {
                let assignment = self.generated_constructor(parser, &fields, end.start);

                if let NodeKind::Assignment(node) = &parser.stream.nodes.get(assignment).kind {
                    constructor = Some(node.init);
                }

                statements.push(assignment);

                None
            }
            None => None,
        };

        if !self.names.contains(CREATE) && (constructor.is_some() || self.base.is_none()) {
            statements.push(self.create(parser, constructor, end.start));
        }

        let statements = parser.stream.nodes.push_list(&statements);
        let body = parser.stream.nodes.push_list(&self.body);

        parser.stream.nodes.close(self.node, end);
        parser.stream.nodes.set_kind(
            self.node,
            Class {
                statements,
                name: self.name,
                base: self.base,
                implements: self.implements,
                members,
                body,
                constructor: written_constructor,
            },
        );
        parser.register_line_item(self.node);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndClassRule;

impl KeywordStatement for EndClassRule {
    fn keywords(&self) -> &[Keyword] {
        &[Keyword::EndClass]
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        parser.parse_block_end(|parser| {
            let Some((node, requires_constructor)) = parser
                .stream
                .blocks
                .top_mut_as::<PendingClass>()
                .map(|class| (class.node, class.requires_constructor(&parser.stream.nodes)))
            else {
                return parser.raise_without_starter(Keyword::EndClass.as_str());
            };
            let end = parser.prev().span();

            if requires_constructor {
                let span = parser.stream.nodes.get(node).span.start;

                parser.report_at(
                    GreybelDiagnostic::InstanceFieldsWithoutConstructor,
                    Span::new(span, end.end),
                )?;
            }

            parser.complete_top_block(&end);
            parser.stream.blocks.push_statement(node);

            Ok(())
        })
    }
}

impl Parser<'_> {
    #[must_use]
    pub(crate) fn is_in_class(&self) -> bool {
        self.stream.blocks.top_is::<PendingClass>()
    }
}
