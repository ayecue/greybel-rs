use greybel_symbols::names::{SELF, SUPER};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Coalesce, Ternary};
use crate::ast::{IdentifierKind, LiteralValue, NodeId, NodeKind, NodeList, NodeStore, UnaryOp};
use crate::position::Span;
use crate::syntax::{
    Assignment, Binary, BinaryOp, Break, ElseClause, Identifier, If, IfClause, IfClauseKind,
    Literal, Parenthesis, Return, Unary, While,
};

#[derive(Debug, Clone, Default)]
pub struct ValueExpansions {
    before: FxHashMap<NodeId, NodeList>,
    instead: FxHashMap<NodeId, NodeList>,
    values: FxHashMap<NodeId, NodeId>,
    blocks: FxHashSet<NodeId>,
}

impl ValueExpansions {
    #[must_use]
    pub fn before(&self, id: NodeId) -> Option<NodeList> {
        self.before.get(&id).copied()
    }

    #[must_use]
    pub fn instead(&self, id: NodeId) -> Option<NodeList> {
        self.instead.get(&id).copied()
    }

    #[must_use]
    pub fn value(&self, id: NodeId) -> Option<NodeId> {
        self.values.get(&id).copied()
    }

    #[must_use]
    pub fn is_block(&self, id: NodeId) -> bool {
        self.blocks.contains(&id)
    }

    pub(crate) fn of(nodes: &mut NodeStore, root: NodeId) -> Self {
        let mut expander = ValueExpander {
            nodes,
            expansions: Self::default(),
            temporaries: 0,
            names: FxHashMap::default(),
            scope: root,
            root,
        };

        expander.walk(root);
        expander.register_names();

        expander.expansions
    }
}

#[derive(Debug, Clone, Copy)]
enum Sink {
    Temporary(usize),
    Assign(NodeId),
    Return,
    Discard,
}

struct ValueExpander<'n> {
    nodes: &'n mut NodeStore,
    expansions: ValueExpansions,
    temporaries: usize,
    names: FxHashMap<NodeId, Vec<usize>>,
    scope: NodeId,
    root: NodeId,
}

impl ValueExpander<'_> {
    const TEMPORARY_PREFIX: &'static str = "__t";

    fn kind(&self, id: NodeId) -> &NodeKind {
        &self.nodes.get(id).kind
    }

    fn children(&self, id: NodeId) -> Vec<NodeId> {
        let mut children = Vec::new();

        self.nodes.for_each_child(id, |child| children.push(child));

        children
    }

    fn walk(&mut self, node: NodeId) {
        for statement in self.statements(node) {
            self.temporaries = 0;
            self.scope = self.nodes.get(statement).scope.unwrap_or(self.root);
            self.expand_statement(statement);
        }

        for child in self.children(node) {
            self.walk(child);
        }

        if let NodeKind::If(statement) = self.kind(node)
            && statement.shortcut
        {
            let clauses = self.nodes.list(statement.clauses).to_vec();
            let expands = clauses.iter().any(|&clause| {
                self.statements(clause).iter().any(|statement| {
                    self.expansions.before.contains_key(statement)
                        || self.expansions.instead.contains_key(statement)
                })
            });

            if expands {
                self.expansions.blocks.insert(node);
                self.expansions.blocks.extend(clauses);
            }
        }
    }

    fn statements(&self, node: NodeId) -> Vec<NodeId> {
        let kind = self.kind(node);
        let list = match kind {
            NodeKind::DestructuringAssignment(assignment) => Some(assignment.statements),
            NodeKind::Switch(switch) => Some(switch.statements),
            NodeKind::Export(export) => Some(export.statements),
            NodeKind::Import(import) => Some(import.bindings),
            _ => kind.as_node().as_block().map(|block| block.body()),
        };

        list.map(|list| self.nodes.list(list).to_vec())
            .unwrap_or_default()
    }

    fn expand_statement(&mut self, statement: NodeId) {
        match self.kind(statement).clone() {
            NodeKind::Assignment(assignment)
                if !assignment.shorthand
                    && self.is_conditional(assignment.init)
                    && self.target_operands(assignment.variable).is_empty() =>
            {
                let statements = self.branch(Sink::Assign(assignment.variable), assignment.init);
                let list = self.nodes.push_list(&statements);

                self.expansions.instead.insert(statement, list);

                return;
            }
            NodeKind::Return(Return {
                argument: Some(argument),
            }) if self.is_conditional(argument) => {
                let statements = self.branch(Sink::Return, argument);
                let list = self.nodes.push_list(&statements);

                self.expansions.instead.insert(statement, list);

                return;
            }
            NodeKind::While(statement_kind) if self.has_values(statement_kind.condition) => {
                let list = self.while_loop(statement_kind.condition, statement_kind.body);

                self.expansions.instead.insert(statement, list);

                return;
            }
            NodeKind::If(statement_kind) => {
                if let Some(list) = self.split_else_if(statement_kind.clauses) {
                    self.expansions.instead.insert(statement, list);

                    return;
                }
            }
            _ if self.is_value(statement) => {
                let statements = self.branch(Sink::Discard, statement);
                let list = self.nodes.push_list(&statements);

                self.expansions.instead.insert(statement, list);

                return;
            }
            _ => {}
        }

        let mut values = Vec::new();

        let headers = self.headers(statement);

        self.collect_in_order(&headers, &mut values);

        if values.is_empty() {
            return;
        }

        let mut before = Vec::new();

        for (value, plain) in values {
            let (statements, temporary) = self.hoist(value, plain);

            before.extend(statements);
            self.expansions.values.insert(value, temporary);
        }

        let list = self.nodes.push_list(&before);

        self.expansions.before.insert(statement, list);
    }

    fn has_values(&self, node: NodeId) -> bool {
        let mut values = Vec::new();

        self.collect(node, false, &mut values);

        !values.is_empty()
    }

    fn while_loop(&mut self, condition: NodeId, body: NodeList) -> NodeList {
        let at = self.nodes.get(condition).span.start;
        let mut values = Vec::new();

        self.collect(condition, false, &mut values);

        let mut statements = Vec::new();

        for (value, plain) in values {
            let (computation, read) = self.hoist(value, plain);

            statements.extend(computation);
            self.expansions.values.insert(value, read);
        }

        let parenthesis = self.alloc(
            Parenthesis {
                expression: condition,
            },
            at,
        );
        let test = self.alloc(
            Unary {
                operator: UnaryOp::Not,
                argument: parenthesis,
            },
            at,
        );
        let exit = self.alloc(Break { loop_node: None }, at);
        let check = self.if_else(test, &[exit], None, at);

        statements.push(check);
        statements.extend_from_slice(self.nodes.list(body));

        let body = self.nodes.push_list(&statements);
        let always = self.alloc(
            Literal {
                value: LiteralValue::Boolean(true),
                negated: false,
            },
            at,
        );
        let statement = self.alloc(
            While {
                condition: always,
                body,
            },
            at,
        );

        self.nodes.push_list(&[statement])
    }

    fn split_else_if(&mut self, clauses: NodeList) -> Option<NodeList> {
        let clauses = self.nodes.list(clauses).to_vec();
        let split = (1..clauses.len()).find(|&index| match self.kind(clauses[index]) {
            NodeKind::IfClause(clause) => self.has_values(clause.condition),
            _ => false,
        })?;
        let NodeKind::IfClause(clause) = self.kind(clauses[split]).clone() else {
            unreachable!("the split clause is an else if");
        };
        let at = self.nodes.get(clauses[split]).span.start;
        let first = self.alloc(
            IfClause {
                kind: IfClauseKind::If,
                shortcut: false,
                ..clause
            },
            at,
        );
        let mut inner_clauses = vec![first];

        inner_clauses.extend_from_slice(&clauses[split + 1..]);

        let inner_clauses = self.nodes.push_list(&inner_clauses);
        let inner = self.alloc(
            If {
                shortcut: false,
                clauses: inner_clauses,
            },
            at,
        );
        let body = self.nodes.push_list(&[inner]);
        let otherwise = self.alloc(
            ElseClause {
                shortcut: false,
                body,
            },
            at,
        );
        let mut outer_clauses = clauses[..split].to_vec();

        outer_clauses.push(otherwise);

        let outer_clauses = self.nodes.push_list(&outer_clauses);
        let outer = self.alloc(
            If {
                shortcut: false,
                clauses: outer_clauses,
            },
            at,
        );

        self.expansions.blocks.extend(clauses);
        self.expand_statement(inner);
        self.expand_statement(outer);

        Some(self.nodes.push_list(&[outer]))
    }

    fn headers(&self, statement: NodeId) -> Vec<(NodeId, bool, bool)> {
        let expressions = match self.kind(statement) {
            NodeKind::Assignment(assignment) => {
                let mut headers = self.target_operands(assignment.variable);

                headers.push((assignment.init, false, true));

                return headers;
            }
            NodeKind::Return(statement) => statement.argument.into_iter().collect(),
            NodeKind::If(statement) => self
                .nodes
                .list(statement.clauses)
                .first()
                .and_then(|&clause| match self.kind(clause) {
                    NodeKind::IfClause(clause) => Some(clause.condition),
                    _ => None,
                })
                .into_iter()
                .collect(),
            NodeKind::For(statement) => vec![statement.sequence],
            NodeKind::Assert(statement) => vec![statement.condition],
            NodeKind::CallStatement(_)
            | NodeKind::Call(_)
            | NodeKind::Identifier(_)
            | NodeKind::Member(_)
            | NodeKind::Index(_)
            | NodeKind::Slice(_)
            | NodeKind::Unary(_)
            | NodeKind::Binary(_)
            | NodeKind::Logical(_)
            | NodeKind::Bitwise(_)
            | NodeKind::Parenthesis(_)
            | NodeKind::List(_)
            | NodeKind::Map(_)
            | NodeKind::Isa(_)
            | NodeKind::ComparisonGroup(_)
            | NodeKind::Template(_) => vec![statement],
            _ => Vec::new(),
        };

        expressions
            .into_iter()
            .map(|expression| (expression, false, true))
            .collect()
    }

    fn target_operands(&self, target: NodeId) -> Vec<(NodeId, bool, bool)> {
        let operand = |node| (!self.is_receiver_name(node)).then_some((node, true, true));

        match self.kind(target) {
            NodeKind::Member(member) => operand(member.base).into_iter().collect(),
            NodeKind::Index(index) => operand(index.base)
                .into_iter()
                .chain([(index.index, false, true)])
                .collect(),
            NodeKind::Unary(unary) => self.target_operands(unary.argument),
            NodeKind::Parenthesis(parenthesis) => self.target_operands(parenthesis.expression),
            _ => Vec::new(),
        }
    }

    fn collect(&self, node: NodeId, plain: bool, values: &mut Vec<(NodeId, bool)>) {
        if self.is_value(node) {
            values.push((node, plain));

            return;
        }

        match self.kind(node) {
            NodeKind::Logical(logical) => self.collect(logical.left, false, values),
            NodeKind::Function(_) => {}
            NodeKind::Parenthesis(parenthesis) => {
                self.collect(parenthesis.expression, plain, values)
            }
            NodeKind::Unary(unary) => {
                self.collect(unary.argument, unary.operator == UnaryOp::AddressOf, values);
            }
            _ => self.collect_children(node, values),
        }
    }

    fn collect_children(&self, node: NodeId, values: &mut Vec<(NodeId, bool)>) {
        let kind = self.kind(node);
        let base = Self::link_base(kind);
        let function = matches!(kind, NodeKind::Call(_)).then_some(base).flatten();
        let children: Vec<(NodeId, bool, bool)> = self
            .children(node)
            .into_iter()
            .map(|child| (child, Some(child) == base, Some(child) != function))
            .collect();

        self.collect_in_order(&children, values);
    }

    fn collect_in_order(
        &self,
        children: &[(NodeId, bool, bool)],
        values: &mut Vec<(NodeId, bool)>,
    ) {
        let last = children
            .iter()
            .rposition(|&(child, ..)| self.has_values(child));

        for (index, &(child, plain, ahead)) in children.iter().enumerate() {
            if ahead && last.is_some_and(|last| index < last) && !self.has_values(child) {
                self.collect_ahead(child, plain, values);
            } else {
                self.collect(child, plain, values);
            }
        }
    }

    fn collect_ahead(&self, node: NodeId, plain: bool, values: &mut Vec<(NodeId, bool)>) {
        match self.kind(node) {
            NodeKind::Literal(_) | NodeKind::Function(_) => {}
            NodeKind::Identifier(identifier) if identifier.kind == IdentifierKind::Property => {}
            NodeKind::ListItem(_) | NodeKind::MapEntry(_) | NodeKind::Spread(_) => {
                for child in self.children(node) {
                    self.collect_ahead(child, false, values);
                }
            }
            _ => values.push((node, plain)),
        }
    }

    fn is_conditional(&self, node: NodeId) -> bool {
        match self.kind(node) {
            NodeKind::Ternary(_) => true,
            NodeKind::Parenthesis(parenthesis) => self.is_conditional(parenthesis.expression),
            _ => false,
        }
    }

    fn is_value(&self, node: NodeId) -> bool {
        matches!(
            self.kind(node),
            NodeKind::Ternary(_) | NodeKind::Coalesce(_)
        ) || self.optional_link(node).is_some()
    }

    fn link_base(kind: &NodeKind) -> Option<NodeId> {
        match kind {
            NodeKind::Member(member) => Some(member.base),
            NodeKind::Index(index) => Some(index.base),
            NodeKind::Slice(slice) => Some(slice.base),
            NodeKind::Call(call) => Some(call.base),
            _ => None,
        }
    }

    fn is_optional(kind: &NodeKind) -> bool {
        match kind {
            NodeKind::Member(member) => member.optional,
            NodeKind::Index(index) => index.optional,
            NodeKind::Slice(slice) => slice.optional,
            NodeKind::Call(call) => call.optional,
            _ => false,
        }
    }

    fn optional_link(&self, node: NodeId) -> Option<(usize, NodeId)> {
        let mut current = match self.kind(node) {
            NodeKind::Unary(unary) if unary.operator == UnaryOp::AddressOf => unary.argument,
            _ => node,
        };
        let mut steps = usize::from(current != node);
        let mut innermost = None;

        while let Some(base) = Self::link_base(self.kind(current)) {
            if Self::is_optional(self.kind(current)) {
                innermost = Some((steps, current));
            }

            current = base;
            steps += 1;
        }

        innermost
    }

    fn step(&self, node: NodeId, steps: usize) -> NodeId {
        let mut current = node;

        for _ in 0..steps {
            current = match self.kind(current) {
                NodeKind::Unary(unary) => unary.argument,
                kind => Self::link_base(kind).expect("the chain has this link"),
            };
        }

        current
    }

    fn hoist(&mut self, value: NodeId, plain: bool) -> (Vec<NodeId>, NodeId) {
        let at = self.nodes.get(value).span.start;
        let temporary = self.next_temporary();
        let source = if self.is_value(value) {
            value
        } else {
            self.nodes.clone_subtree(value)
        };
        let statements = self.branch(Sink::Temporary(temporary), source);
        let read = self.temporary(temporary, at);
        let read = if plain {
            read
        } else {
            self.address_of(read, at)
        };

        (statements, read)
    }

    fn branch(&mut self, sink: Sink, value: NodeId) -> Vec<NodeId> {
        let at = self.nodes.get(value).span.start;

        match self.kind(value).clone() {
            NodeKind::Parenthesis(parenthesis) => return self.branch(sink, parenthesis.expression),
            NodeKind::Ternary(ternary) => {
                return vec![self.ternary_if(sink, &ternary)];
            }
            NodeKind::Coalesce(coalesce) if matches!(sink, Sink::Temporary(_) | Sink::Discard) => {
                return self.coalesce(sink, &coalesce, at);
            }
            _ => {}
        }

        if let Some((steps, link)) = self.optional_link(value)
            && matches!(sink, Sink::Temporary(_) | Sink::Discard)
        {
            return self.chain(sink, value, steps, link, at);
        }

        if self.is_value(value) {
            let (mut statements, read) = self.hoist(value, false);

            statements.push(self.sink_statement(sink, read));

            return statements;
        }

        vec![self.sink_statement(sink, value)]
    }

    fn ternary_if(&mut self, sink: Sink, ternary: &Ternary) -> NodeId {
        let at = self.nodes.get(ternary.condition).span.start;
        let if_true = self.branch(sink, ternary.if_true);
        let if_false = self.branch(sink, ternary.if_false);
        let statement = self.if_else(ternary.condition, &if_true, Some(&if_false), at);

        self.expand_statement(statement);

        statement
    }

    fn coalesce(&mut self, sink: Sink, coalesce: &Coalesce, at: u32) -> Vec<NodeId> {
        let temporary = match sink {
            Sink::Temporary(temporary) => temporary,
            _ => self.next_temporary(),
        };
        let first = self.sink_statement(Sink::Temporary(temporary), coalesce.left);
        let test = self.null_test(temporary, BinaryOp::Equal, at);
        let right = self.branch(sink, coalesce.right);

        vec![first, self.if_else(test, &right, None, at)]
    }

    fn chain(
        &mut self,
        sink: Sink,
        value: NodeId,
        steps: usize,
        link: NodeId,
        at: u32,
    ) -> Vec<NodeId> {
        let temporary = match sink {
            Sink::Temporary(temporary) => temporary,
            _ => self.next_temporary(),
        };
        let link_kind = self.kind(link).clone();
        let base = Self::link_base(&link_kind).expect("an optional link has a base");

        if matches!(link_kind, NodeKind::Call(_)) && matches!(self.kind(base), NodeKind::Member(_))
        {
            return self.method_chain(sink, value, steps, link, at);
        }

        let base = if matches!(link_kind, NodeKind::Call(_)) {
            self.address_of(base, at)
        } else {
            base
        };
        let first = self.sink_statement(Sink::Temporary(temporary), base);
        let remainder = self.nodes.clone_subtree(value);
        let remainder_link = self.step(remainder, steps);
        let read = self.temporary(temporary, at);

        match &mut self.nodes.get_mut(remainder_link).kind {
            NodeKind::Member(member) => (member.base, member.optional) = (read, false),
            NodeKind::Index(index) => (index.base, index.optional) = (read, false),
            NodeKind::Slice(slice) => (slice.base, slice.optional) = (read, false),
            NodeKind::Call(call) => (call.base, call.optional) = (read, false),
            _ => unreachable!("the link is a member, index, slice or call"),
        }

        let test = self.null_test(temporary, BinaryOp::NotEqual, at);
        let inner = self.branch(sink, remainder);

        vec![first, self.if_else(test, &inner, None, at)]
    }

    fn method_chain(
        &mut self,
        sink: Sink,
        value: NodeId,
        steps: usize,
        link: NodeId,
        at: u32,
    ) -> Vec<NodeId> {
        let temporary = match sink {
            Sink::Temporary(temporary) => temporary,
            _ => self.next_temporary(),
        };
        let method = Self::link_base(self.kind(link)).expect("an optional call has a base");
        let object = Self::link_base(self.kind(method)).expect("a method has a receiver");
        let mut statements = Vec::new();
        let receiver = (!self.is_receiver_name(object)).then(|| {
            statements.push(self.sink_statement(Sink::Temporary(temporary), object));

            temporary
        });
        let remainder = self.nodes.clone_subtree(value);
        let remainder_link = self.step(remainder, steps);
        let remainder_method = match &mut self.nodes.get_mut(remainder_link).kind {
            NodeKind::Call(call) => {
                call.optional = false;

                call.base
            }
            _ => unreachable!("the link is a call"),
        };
        let receiver_read = receiver.map(|receiver| self.temporary(receiver, at));

        if let (NodeKind::Member(member), Some(read)) = (
            &mut self.nodes.get_mut(remainder_method).kind,
            receiver_read,
        ) {
            member.base = read;
        }

        let probe = self.nodes.clone_subtree(remainder_method);
        let probe = self.address_of(probe, at);
        let right = self.null_literal(at);
        let test = self.alloc(
            Binary {
                operator: BinaryOp::NotEqual,
                left: probe,
                right,
            },
            at,
        );
        let inner = self.branch(sink, remainder);
        let otherwise = match sink {
            Sink::Temporary(temporary) => {
                let null = self.null_literal(at);

                Some(vec![self.sink_statement(Sink::Temporary(temporary), null)])
            }
            _ => None,
        };

        statements.push(self.if_else(test, &inner, otherwise.as_deref(), at));

        statements
    }

    fn is_receiver_name(&self, node: NodeId) -> bool {
        match self.kind(node) {
            NodeKind::Identifier(identifier) => {
                matches!(self.nodes.resolve(identifier.name), SELF | SUPER)
            }
            _ => false,
        }
    }

    fn sink_statement(&mut self, sink: Sink, value: NodeId) -> NodeId {
        let at = self.nodes.get(value).span.start;
        let statement = match sink {
            Sink::Temporary(temporary) => {
                let variable = self.temporary(temporary, at);

                self.assignment(variable, value, at)
            }
            Sink::Assign(target) => {
                let variable = self.nodes.clone_subtree(target);

                self.assignment(variable, value, at)
            }
            Sink::Return => self.alloc(
                Return {
                    argument: Some(value),
                },
                at,
            ),
            Sink::Discard => value,
        };

        self.expand_statement(statement);

        statement
    }

    fn next_temporary(&mut self) -> usize {
        let temporary = self.temporaries;

        self.temporaries += 1;

        temporary
    }

    fn alloc(&mut self, kind: impl Into<NodeKind>, at: u32) -> NodeId {
        self.nodes.alloc(kind, Span::empty(at), None)
    }

    fn temporary(&mut self, temporary: usize, at: u32) -> NodeId {
        let temporaries = self.names.entry(self.scope).or_default();

        if !temporaries.contains(&temporary) {
            temporaries.push(temporary);
        }

        let name = self
            .nodes
            .insert_atom(&format!("{}{temporary}", Self::TEMPORARY_PREFIX));

        self.alloc(
            Identifier {
                name,
                kind: IdentifierKind::Variable,
            },
            at,
        )
    }

    fn address_of(&mut self, argument: NodeId, at: u32) -> NodeId {
        self.alloc(
            Unary {
                operator: UnaryOp::AddressOf,
                argument,
            },
            at,
        )
    }

    fn assignment(&mut self, variable: NodeId, init: NodeId, at: u32) -> NodeId {
        self.alloc(
            Assignment {
                variable,
                init,
                shorthand: false,
                declaration: None,
            },
            at,
        )
    }

    fn null_test(&mut self, temporary: usize, operator: BinaryOp, at: u32) -> NodeId {
        let read = self.temporary(temporary, at);
        let left = self.address_of(read, at);
        let right = self.null_literal(at);

        self.alloc(
            Binary {
                operator,
                left,
                right,
            },
            at,
        )
    }

    fn null_literal(&mut self, at: u32) -> NodeId {
        self.alloc(
            Literal {
                value: LiteralValue::Nil,
                negated: false,
            },
            at,
        )
    }

    fn if_else(
        &mut self,
        condition: NodeId,
        body: &[NodeId],
        otherwise: Option<&[NodeId]>,
        at: u32,
    ) -> NodeId {
        let body = self.nodes.push_list(body);
        let mut clauses = vec![self.alloc(
            IfClause {
                kind: IfClauseKind::If,
                shortcut: false,
                condition,
                body,
            },
            at,
        )];

        if let Some(otherwise) = otherwise {
            let body = self.nodes.push_list(otherwise);

            clauses.push(self.alloc(
                ElseClause {
                    shortcut: false,
                    body,
                },
                at,
            ));
        }

        let clauses = self.nodes.push_list(&clauses);

        self.alloc(
            If {
                shortcut: false,
                clauses,
            },
            at,
        )
    }

    fn register_names(&mut self) {
        let mut scopes: Vec<(NodeId, Vec<usize>)> =
            std::mem::take(&mut self.names).into_iter().collect();

        scopes.sort_unstable_by_key(|(scope, _)| *scope);

        for (scope, temporaries) in scopes {
            let Some(namespaces) = self
                .kind(scope)
                .as_node()
                .as_scope()
                .map(|data| data.scope().namespaces)
            else {
                continue;
            };
            let mut list = self.nodes.list(namespaces).to_vec();

            for temporary in temporaries {
                list.push(self.temporary(temporary, 0));
            }

            let list = self.nodes.push_list(&list);

            if let Some(data) = self.nodes.get_mut(scope).kind.as_node_mut().as_scope_mut() {
                data.scope_mut().namespaces = list;
            }
        }
    }
}
