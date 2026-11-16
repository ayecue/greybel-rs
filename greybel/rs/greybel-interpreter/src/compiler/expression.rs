use greybel_core::syntax::greybel::Template;
use greybel_core::{BinaryOp, Dialect, LiteralValue, LogicalOp, NodeId, NodeKind, UnaryOp};
use greybel_symbols::names::{GLOBALS, LOCALS, OUTER, SELF, SUPER};

use super::{Compiler, OptionalChain, Usage};
use crate::bytecode::{LookupCache, Op, Operator, Scope, Truthy, UnaryOperator};
use crate::value::{Param, Value};

type Result<T = ()> = super::CompileResult<T>;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Access {
    pub reference: bool,
    pub discard: bool,
    pub optional: bool,
}

impl Access {
    pub const STATEMENT: Self = Self {
        reference: false,
        discard: true,
        optional: false,
    };

    fn optional(optional: bool) -> Self {
        Self {
            optional,
            ..Self::default()
        }
    }
}

impl Compiler<'_> {
    pub fn expression(&mut self, id: NodeId) -> Result {
        self.expression_with_capture(id, false)
    }

    pub(crate) fn expression_with_capture(&mut self, id: NodeId, capture_outer: bool) -> Result {
        if let Some(links) = self.chain_links(id) {
            return self.optional_chain(links, |this| {
                this.expression_with_capture(id, capture_outer)
            });
        }

        match self.kind(id) {
            NodeKind::Member(node) => {
                self.member(node.base, node.identifier, Access::optional(node.optional))
            }
            NodeKind::Index(node) => self.index(
                node.base,
                node.index,
                node.is_statement_start,
                Access::optional(node.optional),
            ),
            NodeKind::Slice(node) => {
                self.slice(id, node.base, node.left, node.right, node.optional)
            }
            NodeKind::Ternary(node) => {
                self.ternary(id, node.condition, node.if_true, node.if_false)
            }
            NodeKind::Coalesce(node) => self.coalesce(id, node.left, node.right),
            NodeKind::TypeAssertion(node) => self.expression(node.expression),
            NodeKind::Identifier(_) => {
                self.identifier(id, Access::default());

                Ok(())
            }
            NodeKind::Isa(node) => self.operation(node.left, node.right, Operator::Isa, id),
            NodeKind::Binary(node) => self.binary(id, node.left, node.right, node.operator),
            NodeKind::Logical(node) => self.logical(id, node.left, node.right, node.operator),
            NodeKind::ComparisonGroup(node) => {
                self.comparison_group(id, self.ast().list(node.expressions), &node.operators)
            }
            NodeKind::Map(node) => self.map(id, self.ast().list(node.fields), false),
            NodeKind::List(node) => self.list(id, self.ast().list(node.fields), false),
            NodeKind::Function(node) => self.function(
                id,
                self.ast().list(node.parameters),
                self.ast().list(node.body),
                capture_outer,
            ),
            NodeKind::Parenthesis(node) => match self.parenthesized_reference(node.expression) {
                Some(target) => self.expression(target),
                None => self.expression_with_capture(node.expression, capture_outer),
            },
            NodeKind::Unary(node) => self.unary(id, node.operator, node.argument, Usage::Value),
            NodeKind::Call(node) => self.call(
                node.base,
                self.ast().list(node.arguments),
                false,
                node.optional,
            ),
            NodeKind::Bitwise(node) => {
                self.operation(node.left, node.right, Operator::Bitwise(node.operator), id)
            }
            NodeKind::ImportCode(node) => self.import_code(id, node.eval, false),
            NodeKind::Template(node) => self.template(id, node),
            NodeKind::Literal(_)
            | NodeKind::Empty(_)
            | NodeKind::Inject(_)
            | NodeKind::Envar(_)
            | NodeKind::Line(_)
            | NodeKind::File(_) => {
                self.push_constant(id);

                Ok(())
            }
            _ => Err(Box::new(self.unexpected_node_error(id))),
        }
    }

    fn push_constant(&mut self, id: NodeId) {
        let value = match self.kind(id) {
            NodeKind::Literal(node) => Value::from_literal(self.ast(), &node.value, node.negated),
            NodeKind::Inject(_) => self
                .graph()
                .module(self.module())
                .injection(id)
                .unwrap_or_default()
                .into(),
            NodeKind::Envar(node) => self.environment(&self.ast().read(node.name)),
            NodeKind::Line(_) => Value::from(f64::from(self.location(id).line)),
            NodeKind::File(node) => Value::from_string(Self::file_name(&node.filename)),
            _ => Value::Null,
        };

        self.push(value, id);
    }

    fn template(&mut self, id: NodeId, node: &Template) -> Result {
        let source = self.ast().source();
        let pieces: Vec<String> = node
            .texts
            .iter()
            .map(|&span| Template::text(source, span).into_owned())
            .collect();
        let expressions = self.ast().list(node.expressions).to_vec();
        let add = Op::binary(BinaryOp::Add.into());

        self.push(Value::from_string(pieces[0].clone()), id);

        for (index, &expression) in expressions.iter().enumerate() {
            self.expression(expression)?;
            self.emit(add.clone(), id);

            let piece = &pieces[index + 1];

            if !piece.is_empty() {
                self.push(Value::from_string(piece.clone()), id);
                self.emit(add.clone(), id);
            }
        }

        Ok(())
    }

    fn slice(
        &mut self,
        id: NodeId,
        base: NodeId,
        left: NodeId,
        right: NodeId,
        optional: bool,
    ) -> Result {
        self.expression(base)?;
        if optional {
            self.optional_check(base);
        }

        self.expression(left)?;
        self.expression(right)?;
        self.emit(Op::Slice, id);

        Ok(())
    }

    fn binary(&mut self, id: NodeId, left: NodeId, right: NodeId, operator: BinaryOp) -> Result {
        match self.constant_value(id) {
            Some(value) => {
                self.push(value, id);

                Ok(())
            }
            None => self.operation(left, right, operator.into(), id),
        }
    }

    fn logical(&mut self, id: NodeId, left: NodeId, right: NodeId, operator: LogicalOp) -> Result {
        let skip = self.label();
        let (when, operator) = match operator {
            LogicalOp::And => (Truthy::False, Operator::And),
            LogicalOp::Or => (Truthy::True, Operator::Or),
        };

        self.expression(left)?;

        let jump = Op::JumpIf {
            target: 0,
            when,
            push_condition: true,
        };

        self.jump(jump, skip, id);
        self.expression(right)?;
        self.emit(Op::binary(operator), id);
        self.set_label(skip);

        Ok(())
    }

    fn comparison_group(
        &mut self,
        id: NodeId,
        expressions: &[NodeId],
        operators: &[BinaryOp],
    ) -> Result {
        for &expression in expressions {
            self.expression(expression)?;
        }

        self.emit(Op::Comparison(operators.into()), id);

        Ok(())
    }

    fn constant_value(&self, id: NodeId) -> Option<Value> {
        match self.kind(self.unwrap(id)) {
            NodeKind::Literal(node) => {
                match Value::from_literal(self.ast(), &node.value, node.negated) {
                    value @ (Value::Number(_) | Value::String(_)) => Some(value),
                    _ => None,
                }
            }
            NodeKind::Binary(node) => {
                let a = self.constant_value(node.left)?;
                let b = self.constant_value(node.right)?;

                Value::scalar_binary(node.operator, &a, &b)
            }
            _ => None,
        }
    }

    fn operation(&mut self, left: NodeId, right: NodeId, operator: Operator, id: NodeId) -> Result {
        self.expression(left)?;
        self.expression(right)?;
        self.emit(Op::binary(operator), id);

        Ok(())
    }

    pub(crate) fn is_super(&self, id: NodeId) -> bool {
        self.name(self.unwrap(id)) == Some(SUPER)
    }

    fn parenthesized_reference(&self, id: NodeId) -> Option<NodeId> {
        if self.options.dialect == Dialect::MiniScript {
            return None;
        }

        let NodeKind::Unary(node) = self.kind(self.unwrap(id)) else {
            return None;
        };
        let target = self.unwrap(node.argument);

        (node.operator == UnaryOp::AddressOf
            && matches!(self.kind(target), NodeKind::Member(_) | NodeKind::Index(_)))
        .then_some(target)
    }

    pub(crate) fn is_self(&self, id: NodeId) -> bool {
        self.name(self.unwrap(id)) == Some(SELF)
    }

    fn is_null(&self, id: NodeId) -> bool {
        matches!(
            self.kind(self.unwrap(id)),
            NodeKind::Literal(node) if node.value == LiteralValue::Nil
        )
    }

    pub(crate) fn member(&mut self, base: NodeId, identifier: NodeId, access: Access) -> Result {
        let name = self.name(identifier).unwrap_or_default();
        let call = (!access.reference).then_some(0);

        if self.is_super(base) {
            let key = Some(self.key(name));

            self.emit(
                Op::GetSuperProperty {
                    key,
                    call,
                    discard: access.discard,
                },
                identifier,
            );
        } else if self.is_self(base) {
            let key = Some(self.key(name));

            self.emit(
                Op::GetSelfProperty {
                    key,
                    call,
                    discard: access.discard,
                },
                identifier,
            );
        } else if self.is_null(base) && !access.optional {
            self.push(Value::Null, base);
            self.push(name, identifier);
            self.emit(
                Op::GetProperty {
                    call,
                    discard: access.discard,
                },
                identifier,
            );
        } else {
            self.expression(self.unwrap(base))?;
            if access.optional {
                self.optional_check(base);
            }

            let key = self.key(name);

            self.emit(
                Op::GetMember {
                    key,
                    call,
                    discard: access.discard,
                },
                identifier,
            );
        }

        Ok(())
    }

    pub(crate) fn index(
        &mut self,
        base: NodeId,
        index: NodeId,
        statement_start: bool,
        access: Access,
    ) -> Result {
        let invoke = if access.discard {
            !access.reference
        } else {
            statement_start && !access.reference
        };

        if self.is_super(base) {
            self.expression(index)?;

            let call = (access.discard && invoke).then_some(0);

            self.emit(
                Op::GetSuperProperty {
                    key: None,
                    call,
                    discard: access.discard,
                },
                index,
            );
        } else if self.is_self(base) {
            self.expression(index)?;
            self.emit(
                Op::GetSelfProperty {
                    key: None,
                    call: invoke.then_some(0),
                    discard: access.discard,
                },
                index,
            );
        } else {
            self.expression(self.unwrap(base))?;
            if access.optional {
                self.optional_check(base);
            }

            self.expression(index)?;
            self.emit(
                Op::GetProperty {
                    call: invoke.then_some(0),
                    discard: access.discard,
                },
                index,
            );
        }

        Ok(())
    }

    pub(crate) fn identifier(&mut self, id: NodeId, access: Access) {
        let name = self.name(id).unwrap_or_default();

        if let Some(slot) = self.slot(name) {
            let receiver = matches!(name, SELF | SUPER);

            if receiver && access.discard {
                return;
            }

            let name = self.key(name);

            self.emit(
                Op::GetSlot {
                    slot,
                    key: name,
                    invoke: !receiver && !access.reference,
                    discard: access.discard,
                },
                id,
            );

            return;
        }

        let scope = match name {
            OUTER => Some(Scope::Outer),
            LOCALS => Some(Scope::Locals),
            GLOBALS => Some(Scope::Globals),
            _ => None,
        };
        let receiver = matches!(name, SELF | SUPER);

        if (scope.is_some() || receiver) && access.discard {
            return;
        }

        if let Some(scope) = scope {
            self.emit(Op::GetScope(scope), id);

            return;
        }

        let name = self.key(name);

        self.emit(
            Op::GetVariable {
                key: name,
                invoke: !receiver && !access.reference,
                discard: access.discard,
                cache: LookupCache::default(),
            },
            id,
        );
    }

    pub(crate) fn map(&mut self, id: NodeId, fields: &[NodeId], discard: bool) -> Result {
        if self.has_spread(fields) {
            return self.spread_collection(id, fields, discard, true);
        }

        for &field in fields {
            let NodeKind::MapEntry(entry) = self.kind(field) else {
                return Err(Box::new(self.unexpected_node_error(field)));
            };

            self.expression(entry.key)?;
            self.expression(entry.value)?;
        }

        let entries = u32::try_from(fields.len()).expect("fewer than 2^32 entries");

        self.emit(Op::MakeMap { entries, discard }, id);

        Ok(())
    }

    pub(crate) fn list(&mut self, id: NodeId, fields: &[NodeId], discard: bool) -> Result {
        if self.has_spread(fields) {
            return self.spread_collection(id, fields, discard, false);
        }

        for &field in fields {
            let NodeKind::ListItem(item) = self.kind(field) else {
                return Err(Box::new(self.unexpected_node_error(field)));
            };

            self.expression(item.value)?;
        }

        let items = u32::try_from(fields.len()).expect("fewer than 2^32 items");

        self.emit(Op::MakeList { items, discard }, id);

        Ok(())
    }

    fn has_spread(&self, fields: &[NodeId]) -> bool {
        fields
            .iter()
            .any(|&field| matches!(self.kind(field), NodeKind::Spread(_)))
    }

    fn spread_collection(
        &mut self,
        id: NodeId,
        fields: &[NodeId],
        discard: bool,
        map: bool,
    ) -> Result {
        let add = Op::binary(BinaryOp::Add.into());
        let empty = |this: &mut Self, fields: &[NodeId]| {
            if map {
                this.map(id, fields, false)
            } else {
                this.list(id, fields, false)
            }
        };

        empty(self, &[])?;

        let spreads: Vec<Option<NodeId>> = fields
            .iter()
            .map(|&field| match self.kind(field) {
                NodeKind::Spread(spread) => Some(spread.argument),
                _ => None,
            })
            .collect();
        let mut index = 0;

        while index < fields.len() {
            if let Some(argument) = spreads[index] {
                self.expression(argument)?;
                index += 1;
            } else {
                let end = spreads[index..]
                    .iter()
                    .position(Option::is_some)
                    .map_or(fields.len(), |offset| index + offset);

                empty(self, &fields[index..end])?;
                index = end;
            }

            self.emit(add.clone(), id);
        }

        if discard {
            self.emit(Op::Pop, id);
        }

        Ok(())
    }

    fn params(&mut self, parameters: &[NodeId]) -> Vec<Param> {
        parameters
            .iter()
            .map(|&parameter| match self.kind(parameter) {
                NodeKind::Assignment(node) => {
                    let name = self.name(node.variable).unwrap_or_default();
                    let default = match self.kind(node.init) {
                        NodeKind::Literal(literal) => {
                            Value::from_literal(self.ast(), &literal.value, literal.negated)
                        }
                        _ => Value::Null,
                    };

                    Param::with_default(self.shared_str(name), default)
                }
                _ => {
                    let name = self.name(parameter).unwrap_or_default();

                    Param::new(self.shared_str(name))
                }
            })
            .collect()
    }

    fn function(
        &mut self,
        id: NodeId,
        parameters: &[NodeId],
        body: &[NodeId],
        capture_outer: bool,
    ) -> Result {
        let params = self.params(parameters);
        let layout = self.slot_layout(id, &params);
        let code = self.add_code_builder(false, |c| {
            if let Some(layout) = layout {
                c.use_slots(layout, &params);
            }

            c.expression_defaults(parameters)?;
            c.body(body)?;
            c.emit_internal(Op::Return(Some(Value::Null)));

            Ok(())
        })?;

        self.emit(
            Op::Function {
                params: params.into(),
                code,
                capture_outer,
            },
            id,
        );

        Ok(())
    }

    pub(crate) fn unary(
        &mut self,
        id: NodeId,
        operator: UnaryOp,
        argument: NodeId,
        usage: Usage,
    ) -> Result {
        let discard = usage == Usage::Statement;
        let argument = self.unwrap(argument);
        let op = match operator {
            UnaryOp::AddressOf => {
                let access = Access {
                    reference: true,
                    discard,
                    optional: false,
                };

                return match self.kind(argument) {
                    NodeKind::Member(node) => self.member(
                        node.base,
                        node.identifier,
                        Access {
                            optional: node.optional,
                            ..access
                        },
                    ),
                    NodeKind::Index(node) => {
                        let access = if discard { access } else { Access::default() };
                        let access = Access {
                            optional: node.optional,
                            ..access
                        };

                        self.index(node.base, node.index, node.is_statement_start, access)
                    }
                    NodeKind::Identifier(_) => {
                        self.identifier(argument, access);

                        Ok(())
                    }
                    _ if discard => self.statement(argument),
                    _ => self.expression(argument),
                };
            }
            UnaryOp::Not => UnaryOperator::Not,
            UnaryOp::Negate => UnaryOperator::Negate,
            UnaryOp::New => UnaryOperator::New,
        };
        let op = Op::Unary {
            operator: op,
            discard,
        };

        self.expression(argument)?;
        self.emit(op, id);

        Ok(())
    }

    pub(crate) fn call(
        &mut self,
        base: NodeId,
        arguments: &[NodeId],
        discard: bool,
        optional: bool,
    ) -> Result {
        let count = u32::try_from(arguments.len()).expect("fewer than 2^32 arguments");
        let (op, at) = if optional {
            self.optional_callee(base, count, discard)?
        } else {
            self.callee(base, count, discard)?
        };

        for &argument in arguments {
            self.expression(argument)?;
        }

        self.emit(op, at);

        Ok(())
    }

    fn callee(&mut self, base: NodeId, count: u32, discard: bool) -> Result<(Op, NodeId)> {
        let callee = self.unwrap(base);
        let call = Some(count);

        match self.kind(callee) {
            NodeKind::Member(node) if node.optional => {
                self.expression(self.unwrap(node.base))?;
                self.optional_check(node.base);

                let key = self.key(self.name(node.identifier).unwrap_or_default());

                Ok((Op::GetMember { key, call, discard }, node.identifier))
            }
            NodeKind::Index(node) if node.optional => {
                self.expression(self.unwrap(node.base))?;
                self.optional_check(node.base);
                self.expression(node.index)?;

                Ok((Op::GetProperty { call, discard }, node.index))
            }
            NodeKind::Member(node) if self.is_super(node.base) => {
                let key = Some(self.key(self.name(node.identifier).unwrap_or_default()));
                let op = Op::GetSuperProperty { key, call, discard };

                Ok((op, base))
            }
            NodeKind::Member(node) if self.is_self(node.base) => {
                let key = Some(self.key(self.name(node.identifier).unwrap_or_default()));
                let op = Op::GetSelfProperty { key, call, discard };

                Ok((op, node.identifier))
            }
            NodeKind::Member(node) if self.is_null(node.base) => {
                self.push(Value::Null, node.base);
                self.push(
                    self.name(node.identifier).unwrap_or_default(),
                    node.identifier,
                );

                Ok((Op::GetProperty { call, discard }, node.identifier))
            }
            NodeKind::Member(node) => {
                self.expression(self.unwrap(node.base))?;

                let key = self.key(self.name(node.identifier).unwrap_or_default());

                Ok((Op::GetMember { key, call, discard }, node.identifier))
            }
            NodeKind::Index(node) if self.is_super(node.base) => {
                self.expression(node.index)?;

                let op = Op::GetSuperProperty {
                    key: None,
                    call,
                    discard,
                };

                Ok((op, node.index))
            }
            NodeKind::Index(node) if self.is_self(node.base) => {
                self.expression(node.index)?;

                let op = Op::GetSelfProperty {
                    key: None,
                    call,
                    discard,
                };

                Ok((op, node.index))
            }
            NodeKind::Index(node) => {
                self.expression(self.unwrap(node.base))?;
                self.expression(node.index)?;

                Ok((Op::GetProperty { call, discard }, node.index))
            }
            NodeKind::Identifier(_) => {
                let access = Access {
                    reference: true,
                    discard: false,
                    optional: false,
                };

                self.identifier(callee, access);

                let op = Op::Call {
                    arguments: count,
                    discard,
                };

                Ok((op, callee))
            }
            _ => {
                self.expression(callee)?;

                let op = Op::Call {
                    arguments: count,
                    discard,
                };

                Ok((op, callee))
            }
        }
    }

    fn optional_callee(&mut self, base: NodeId, count: u32, discard: bool) -> Result<(Op, NodeId)> {
        let callee = self.unwrap(base);
        let call = Some(count);

        match self.kind(callee) {
            NodeKind::Member(node) if self.is_self(node.base) || self.is_super(node.base) => {
                let key = Some(self.key(self.name(node.identifier).unwrap_or_default()));
                let receiver_property = |key, call, discard| {
                    if self.is_super(node.base) {
                        Op::GetSuperProperty { key, call, discard }
                    } else {
                        Op::GetSelfProperty { key, call, discard }
                    }
                };
                let read = receiver_property(key.clone(), None, false);
                let op = receiver_property(key, call, discard);

                self.emit(read, node.identifier);
                self.optional_method_check(callee, 1);

                Ok((op, node.identifier))
            }
            NodeKind::Member(node) => {
                self.expression(self.unwrap(node.base))?;
                if node.optional {
                    self.optional_check(node.base);
                }

                let key = self.key(self.name(node.identifier).unwrap_or_default());

                self.emit(Op::Duplicate(1), node.identifier);
                self.emit(
                    Op::GetMember {
                        key: key.clone(),
                        call: None,
                        discard: false,
                    },
                    node.identifier,
                );
                self.optional_method_check(callee, 2);

                Ok((Op::GetMember { key, call, discard }, node.identifier))
            }
            NodeKind::Index(node) => {
                let reference = Access {
                    reference: true,
                    discard: false,
                    optional: node.optional,
                };

                self.index(node.base, node.index, false, reference)?;
                self.optional_check(callee);

                Ok((
                    Op::Call {
                        arguments: count,
                        discard,
                    },
                    callee,
                ))
            }
            NodeKind::Identifier(_) => {
                let reference = Access {
                    reference: true,
                    ..Access::default()
                };

                self.identifier(callee, reference);
                self.optional_check(callee);

                Ok((
                    Op::Call {
                        arguments: count,
                        discard,
                    },
                    callee,
                ))
            }
            _ => {
                self.expression(callee)?;
                self.optional_check(callee);

                Ok((
                    Op::Call {
                        arguments: count,
                        discard,
                    },
                    callee,
                ))
            }
        }
    }

    fn optional_method_check(&mut self, at: NodeId, values: u8) {
        let end = self
            .chains
            .last()
            .map(|chain| chain.end)
            .expect("a `?.` call is compiled in its chain");
        let present = self.label();

        self.null_check(at);
        self.jump(Self::jump_if_false(), present, at);

        for _ in 0..values {
            self.emit(Op::Pop, at);
        }

        self.push(Value::Null, at);
        self.jump(Op::Jump(0), end, at);
        self.set_label(present);
        self.emit(Op::Pop, at);
    }

    fn ternary(
        &mut self,
        id: NodeId,
        condition: NodeId,
        if_true: NodeId,
        if_false: NodeId,
    ) -> Result {
        let otherwise = self.label();
        let end = self.label();

        self.expression(condition)?;
        self.jump(Self::jump_if_false(), otherwise, id);
        self.expression(if_true)?;
        self.jump(Op::Jump(0), end, id);
        self.set_label(otherwise);
        self.expression(if_false)?;
        self.set_label(end);

        Ok(())
    }

    fn coalesce(&mut self, id: NodeId, left: NodeId, right: NodeId) -> Result {
        let end = self.label();

        self.expression(left)?;
        self.null_check(id);
        self.jump(Self::jump_if_false(), end, id);
        self.emit(Op::Pop, id);
        self.expression(right)?;
        self.set_label(end);

        Ok(())
    }

    fn null_check(&mut self, at: NodeId) {
        self.emit(Op::Duplicate(1), at);
        self.push(Value::Null, at);
        self.emit(Op::binary(Operator::from(BinaryOp::Equal)), at);
    }

    fn optional_check(&mut self, at: NodeId) {
        let end = self
            .chains
            .last()
            .map(|chain| chain.end)
            .expect("a `?.` link is compiled in its chain");
        let jump = Op::JumpIf {
            target: 0,
            when: Truthy::True,
            push_condition: false,
        };

        self.null_check(at);
        self.jump(jump, end, at);
    }

    pub(crate) fn chain_links(&self, id: NodeId) -> Option<Vec<NodeId>> {
        if self
            .chains
            .last()
            .is_some_and(|chain| chain.links.contains(&id))
        {
            return None;
        }

        let mut links = vec![id];
        let mut current = match self.kind(id) {
            NodeKind::Unary(node) if node.operator == UnaryOp::AddressOf => node.argument,
            _ => id,
        };
        let mut optional = false;

        loop {
            let (base, link_optional) = match self.kind(current) {
                NodeKind::Member(node) => (node.base, node.optional),
                NodeKind::Index(node) => (node.base, node.optional),
                NodeKind::Slice(node) => (node.base, node.optional),
                NodeKind::Call(node) => (node.base, node.optional),
                _ => break,
            };

            links.push(current);
            optional |= link_optional;
            current = base;
        }

        optional.then_some(links)
    }

    pub(crate) fn optional_chain(
        &mut self,
        links: Vec<NodeId>,
        compile: impl FnOnce(&mut Self) -> Result,
    ) -> Result {
        let end = self.label();

        self.chains.push(OptionalChain { end, links });

        let result = compile(self);

        self.chains.pop();
        self.set_label(end);

        result
    }
}
