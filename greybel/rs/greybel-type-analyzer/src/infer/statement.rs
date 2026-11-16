use std::{iter, mem};

use greybel_core::{NodeId, NodeKind};
use greybel_symbols::Keyword;
use greybel_symbols::annotations::Tag;
use greybel_symbols::names::{ENTRY_KEY, ENTRY_VALUE, LOOP_INDEX_PREFIX, LOOP_INDEX_SUFFIX};

use super::Inference;
use super::flow::{FactTable, Guard};
use crate::diagnostic::DiagnosticKind;
use crate::model::{KeyRef, MapOrigin, ScopeId};
use crate::types::{Type, TypeId};

const MAX_LOOP_ITERATIONS: usize = 8;

impl<'a> Inference<'a> {
    pub(crate) fn infer_block(&mut self, body: &[NodeId]) {
        let mut terminator = None;
        let mut is_reported = false;

        for &statement in body {
            if matches!(
                self.ast().kind(statement),
                NodeKind::Noop(_) | NodeKind::TypeStatement(_)
            ) {
                continue;
            }

            // A statement a build adds (an empty span, such as a constructor's `return self`) isn't written code.
            if let Some(terminator) = terminator
                && !is_reported
                && !self.ast().span(statement).is_empty()
            {
                self.report(
                    DiagnosticKind::DeadCodeAfterTerminator,
                    statement,
                    format!("Dead code after {terminator}."),
                );
                is_reported = true;
            }

            let was_reachable = self.flow.is_reachable();

            self.infer_statement(statement);
            if was_reachable && !self.flow.is_reachable() {
                terminator = Some(self.terminator_name(statement));
            }
        }
    }

    fn terminator_name(&self, statement: NodeId) -> &'a str {
        let ast = self.ast();
        let callee_name = |expression: NodeId| {
            let callee = match ast.kind(expression) {
                NodeKind::Call(call) => call.base,
                _ => expression,
            };

            ast.text(callee).unwrap_or(Keyword::Function.as_str())
        };

        match ast.kind(statement) {
            NodeKind::Return(_) => Keyword::Return.as_str(),
            NodeKind::Break(_) => Keyword::Break.as_str(),
            NodeKind::Continue(_) => Keyword::Continue.as_str(),
            NodeKind::If(_) => Keyword::If.as_str(),
            NodeKind::While(_) => Keyword::While.as_str(),
            NodeKind::For(_) => Keyword::For.as_str(),
            NodeKind::CallStatement(statement) => callee_name(statement.expression),
            NodeKind::Assignment(assignment) => callee_name(assignment.init),
            _ => callee_name(statement),
        }
    }

    /// Greybel's statement sugar: an `enum`, a destructuring assignment and a class as the statements they stand for, and an
    /// `assert` as a narrowing by its condition.
    fn infer_greybel_statement(&mut self, node: NodeId) {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Enum(enumeration) => {
                self.infer_statement(enumeration.assignment);
                self.register_enum_type(enumeration);
            }
            NodeKind::DestructuringAssignment(assignment) => {
                for &statement in ast.list(assignment.statements) {
                    self.infer_statement(statement);
                }
            }
            NodeKind::Switch(switch) => {
                for &statement in ast.list(switch.statements) {
                    self.infer_statement(statement);
                }

                self.validate_switch_cases(node, switch);
            }
            NodeKind::Export(export) => {
                for &statement in ast.list(export.statements) {
                    self.infer_statement(statement);
                }
            }
            NodeKind::Class(class) => {
                for &statement in ast.list(class.statements) {
                    self.infer_statement(statement);
                }

                self.register_class(class);
            }
            NodeKind::Assert(assert) => {
                self.infer_value(assert.condition);

                let guard = self.condition_guard(assert.condition);

                if let Some(message) = assert.message {
                    self.infer_value(message);
                }

                self.refine_flow(&guard.when_true);
            }
            _ => {}
        }
    }

    pub(crate) fn infer_statement(&mut self, node: NodeId) {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Assignment(assignment) => {
                self.validate_self_assignment(assignment.variable, assignment.init);
                self.custom_map = self.custom_map_at(node, assignment.init);

                let mut value = self.infer_value(assignment.init);

                self.custom_map = None;
                if let Some(annotation_type) = self.annotated_type_at(node) {
                    self.validate_declared_type(assignment.variable, annotation_type, value);
                    value = annotation_type;
                }

                self.assign(assignment.variable, value, assignment.init);
            }
            NodeKind::CallStatement(statement) => {
                self.infer_value(statement.expression);
            }
            NodeKind::Return(statement) => {
                let value = match statement.argument {
                    Some(argument) => self.infer_value(argument),
                    None => TypeId::NULL,
                };

                if let ScopeId::Function(function) = self.current_scope {
                    let data = self.model.function(function);
                    let is_annotated = data.doc.as_ref().is_some_and(|doc| doc.returns.is_some());
                    let (slot, site) = (data.return_slot, data.site);

                    if is_annotated {
                        let name = self.function_name(site.node);
                        let annotation_type = self.model.slot_type(slot);

                        self.validate_against_annotation(
                            node,
                            Tag::Returns,
                            &name,
                            annotation_type,
                            value,
                        );
                    } else {
                        self.widen(slot, value);
                    }
                }

                self.flow.set_unreachable();
            }
            NodeKind::If(statement) => self.infer_if(ast.list(statement.clauses)),
            NodeKind::While(statement) => {
                let condition = statement.condition;

                self.infer_loop(node, ast.list(statement.body), |this| {
                    this.infer_value(condition);

                    (
                        this.condition_guard(condition),
                        this.is_always_true(condition),
                    )
                });
            }
            NodeKind::For(statement) => {
                let iterable = self.infer_value(statement.sequence);
                let item = self.iteration_item_type(node, iterable);

                let index = self.identifier_name(statement.variable).map(|name| {
                    let index = self.model.insert_variable(
                        self.current_scope,
                        &format!("{LOOP_INDEX_PREFIX}{name}{LOOP_INDEX_SUFFIX}"),
                    );

                    self.widen(index, TypeId::NUMBER);

                    index
                });

                self.infer_loop(node, ast.list(statement.body), |this| {
                    this.assign(statement.variable, item, statement.sequence);
                    if let Some(index) = index {
                        this.add_assignment(this.current_scope, index);
                        this.flow.assign(index, TypeId::NUMBER);
                    }

                    (Guard::default(), false)
                });
            }
            NodeKind::Enum(_)
            | NodeKind::DestructuringAssignment(_)
            | NodeKind::Switch(_)
            | NodeKind::Export(_)
            | NodeKind::Class(_)
            | NodeKind::Assert(_) => {
                self.infer_greybel_statement(node);
            }
            NodeKind::Break(_) => self.exit_iteration(true),
            NodeKind::Continue(_) => self.exit_iteration(false),
            NodeKind::Noop(_) | NodeKind::Invalid(_) | NodeKind::TypeStatement(_) => {}
            NodeKind::Import(_)
            | NodeKind::Include(_)
            | NodeKind::ImportCode(_)
            | NodeKind::Envar(_)
            | NodeKind::Inject(_)
            | NodeKind::File(_)
            | NodeKind::Line(_)
            | NodeKind::Debugger(_) => {
                self.infer_directive(node);
            }
            _ => {
                self.infer_value(node);
            }
        }
    }

    fn infer_if(&mut self, clauses: &[NodeId]) {
        let ast = self.ast();
        let entry = self.flow.clone();
        let mut previous_false = FactTable::default();
        let mut ends = Vec::new();
        let mut has_else = false;

        for &clause in clauses {
            self.flow = self.with_facts(&entry, &previous_false);

            match ast.kind(clause) {
                NodeKind::IfClause(clause) => {
                    self.infer_value(clause.condition);
                    self.validate_condition(clause.condition);

                    let guard = self.condition_guard(clause.condition);

                    self.refine_flow(&guard.when_true);
                    self.infer_block(ast.list(clause.body));
                    previous_false =
                        previous_false.intersection(&guard.when_false, &mut self.types);
                }
                NodeKind::ElseClause(clause) => {
                    self.infer_block(ast.list(clause.body));
                    has_else = true;
                }
                _ => continue,
            }

            ends.push(mem::take(&mut self.flow));
        }

        if !has_else {
            ends.push(self.with_facts(&entry, &previous_false));
        }

        self.flow = self.join_all_flows(ends);
    }

    fn infer_loop(
        &mut self,
        node: NodeId,
        body: &[NodeId],
        mut start_iteration: impl FnMut(&mut Self) -> (Guard, bool),
    ) {
        let key = (self.site(node), self.context.current);
        let entry = mem::take(&mut self.flow);
        let mut header = match self.previous.loop_back_states.get(&key).cloned() {
            Some(back) => self.join_flows(entry.clone(), &back),
            None => entry.clone(),
        };
        let reports = (
            self.pass.diagnostics.len(),
            self.pass.deferred_diagnostics.len(),
        );
        let visited_documents = self.pass.visited_documents.len();
        let node_types = self.node_type_log.len();
        let mut iteration = 1;
        let max_iterations = if self.pass.number == 0 {
            1
        } else {
            MAX_LOOP_ITERATIONS
        };

        if max_iterations > 1 {
            self.repeatable_loops += 1;
        }

        loop {
            self.flow = header.clone();

            let (guard, is_endless) = start_iteration(self);

            self.enter_loop();
            self.refine_flow(&guard.when_true);
            self.infer_block(body);

            let frame = self.exit_loop();
            let end = mem::take(&mut self.flow);
            let back = self.join_all_flows(iter::once(end).chain(frame.continues));
            let next_header = self.join_flows(entry.clone(), &back);
            let is_stable = next_header == header;

            if is_stable
                || iteration == max_iterations
                || self.pass.visited_documents.len() != visited_documents
            {
                if !is_stable {
                    self.pass.has_changes = true;
                }

                self.previous.loop_back_states.insert(key, back);

                let mut exits = frame.breaks;

                if !is_endless {
                    exits.push(self.with_facts(&header, &guard.when_false));
                }

                self.flow = self.join_all_flows(exits);
                if max_iterations > 1 {
                    self.repeatable_loops -= 1;
                    if self.repeatable_loops == 0 {
                        self.node_type_log.clear();
                    }
                }

                return;
            }

            self.pass.diagnostics.truncate(reports.0);
            self.pass.deferred_diagnostics.truncate(reports.1);
            for (document, node, node_type) in self.node_type_log.drain(node_types..).rev() {
                self.node_types[document as usize][node.index()] = node_type;
            }

            header = next_header;
            iteration += 1;
        }
    }

    fn iteration_item_type(&mut self, node: NodeId, iterable: TypeId) -> TypeId {
        let iterable = self.types.erase_type_parameters(iterable);
        let variants: Vec<TypeId> = self.types.variants(iterable).collect();
        let items: Vec<TypeId> = variants
            .into_iter()
            .map(
                |variant| match self.types.get(self.types.without_literal(variant)).clone() {
                    Type::List(item) => item,
                    Type::String => TypeId::STRING,
                    Type::Any => TypeId::ANY,
                    Type::Map { key, value } => self.key_value_pair_type(node, key, value),
                    Type::Object(map) => {
                        let (key, value) = self.entry_types(map);

                        self.key_value_pair_type(node, key, value)
                    }
                    Type::Class(_) => self.key_value_pair_type(node, TypeId::ANY, TypeId::ANY),
                    _ => TypeId::UNKNOWN,
                },
            )
            .collect();

        self.types.union_all(items)
    }

    fn key_value_pair_type(&mut self, node: NodeId, key: TypeId, value: TypeId) -> TypeId {
        let map = self
            .model
            .insert_map_in(MapOrigin::Iteration(self.site(node)), self.context.current);
        let object = self.object_type(map);

        self.assign_entry(object, KeyRef::String(ENTRY_KEY), key);
        self.assign_entry(object, KeyRef::String(ENTRY_VALUE), value);

        object
    }
}
