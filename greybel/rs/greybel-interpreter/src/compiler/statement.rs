use greybel_core::{Dialect, Keyword, NodeId, NodeKind};
use greybel_documents::NativeImportMode;

use super::expression::Access;
use super::{Compiler, Label, Usage};
use crate::bytecode::{EntryCache, Local, LookupCache, Op, Operand, Operator, Truthy};
use crate::value::Value;

type Result<T = ()> = super::CompileResult<T>;

impl Compiler<'_> {
    pub fn statement(&mut self, id: NodeId) -> Result {
        if self.debug() {
            self.breakpoint(id);
        }

        match self.kind(id) {
            NodeKind::Chunk(node) => self.body(self.ast().list(node.body)),
            NodeKind::Member(node) => self.member(node.base, node.identifier, Access::STATEMENT),
            NodeKind::Index(node) => self.index(node.base, node.index, true, Access::STATEMENT),
            NodeKind::Identifier(_) => {
                self.identifier(id, Access::STATEMENT);

                Ok(())
            }
            NodeKind::Assignment(node) if node.shorthand && self.evaluates_targets_once() => {
                self.shorthand_assignment(id, node.variable, node.init)
            }
            NodeKind::Assignment(node) => self.assignment(id, node.variable, node.init),
            NodeKind::Isa(_)
            | NodeKind::Binary(_)
            | NodeKind::Logical(_)
            | NodeKind::ComparisonGroup(_)
            | NodeKind::Bitwise(_) => self.expression_statement(id),
            NodeKind::Return(node) => self.return_statement(id, node.argument),
            NodeKind::Break(_) => {
                self.break_statement(id);

                Ok(())
            }
            NodeKind::Continue(_) => {
                self.continue_statement(id);

                Ok(())
            }
            NodeKind::Map(node) => self.map(id, self.ast().list(node.fields), true),
            NodeKind::List(node) => self.list(id, self.ast().list(node.fields), true),
            NodeKind::Parenthesis(node) => self.statement(node.expression),
            NodeKind::Unary(node) => self.unary(id, node.operator, node.argument, Usage::Statement),
            NodeKind::CallStatement(node) => match self.kind(node.expression) {
                NodeKind::Call(call) => self.call(call.base, self.ast().list(call.arguments), true),
                _ => self.statement(node.expression),
            },
            NodeKind::Call(node) => self.call(node.base, self.ast().list(node.arguments), true),
            NodeKind::While(node) => self.while_loop(node.condition, self.ast().list(node.body)),
            NodeKind::If(node) => self.if_statement(id, self.ast().list(node.clauses)),
            NodeKind::For(node) => {
                self.for_loop(id, node.variable, node.sequence, self.ast().list(node.body))
            }
            NodeKind::Import(node) => self.import_statement(id, node.name),
            NodeKind::Include(node) => self.include_statement(id, node.type_only),
            NodeKind::Debugger(_) => {
                self.debugger(id);

                Ok(())
            }
            NodeKind::ImportCode(node) => self.import_code(id, node.eval, true),
            NodeKind::Literal(_)
            | NodeKind::Slice(_)
            | NodeKind::Function(_)
            | NodeKind::Empty(_)
            | NodeKind::Noop(_)
            | NodeKind::Unknown(_)
            | NodeKind::Invalid(_)
            | NodeKind::Inject(_)
            | NodeKind::Envar(_)
            | NodeKind::Line(_)
            | NodeKind::File(_) => Ok(()),
            _ => Err(Box::new(self.unexpected_node_error(id))),
        }
    }

    fn breakpoint(&mut self, id: NodeId) {
        self.emit(Op::Breakpoint, id);
    }

    fn debugger(&mut self, id: NodeId) {
        self.emit(Op::EnableBreakpoint, id);
        self.emit(Op::Breakpoint, id);
    }

    fn expression_statement(&mut self, id: NodeId) -> Result {
        self.expression(id)?;
        self.emit(Op::Pop, id);

        Ok(())
    }

    fn break_statement(&mut self, id: NodeId) {
        if let Some(&(_, exit)) = self.loops().last() {
            self.jump(Op::Jump(0), exit, id);
        }
    }

    fn continue_statement(&mut self, id: NodeId) {
        if let Some(&(next, _)) = self.loops().last() {
            self.jump(Op::Jump(0), next, id);
        }
    }

    fn import_statement(&mut self, id: NodeId, name: NodeId) -> Result {
        let Some(target) = self.dependency(id) else {
            return Ok(());
        };
        let (slot, code) = self.import(target)?;
        let name = self.key(self.name(name).unwrap_or_default());

        self.emit(Op::Import { slot, code }, id);
        self.emit(Op::Export(slot), id);
        self.emit(
            Op::AssignLocal {
                key: name,
                cache: EntryCache::default(),
            },
            id,
        );

        Ok(())
    }

    fn include_statement(&mut self, id: NodeId, type_only: bool) -> Result {
        if type_only {
            return Ok(());
        }

        match self.dependency(id) {
            Some(target) => self.include(target),
            None => Ok(()),
        }
    }

    pub(crate) fn import_code(&mut self, id: NodeId, eval: bool, discard: bool) -> Result {
        if !eval {
            return Ok(());
        }

        if self.options.native_imports == NativeImportMode::Call {
            let key = self.key(Keyword::ImportCode.as_str());

            self.emit(
                Op::GetVariable {
                    key,
                    invoke: true,
                    discard,
                    cache: LookupCache::default(),
                },
                id,
            );

            return Ok(());
        }

        let Some(target) = self.dependency(id) else {
            return Ok(());
        };

        if !self.included_native_imports.insert(target) {
            return Ok(());
        }

        self.include(target)
    }

    fn assignment(&mut self, id: NodeId, variable: NodeId, init: NodeId) -> Result {
        let op = self.assignment_target(variable)?;

        self.expression_with_capture(init, true)?;
        self.emit(op, id);

        Ok(())
    }

    fn evaluates_targets_once(&self) -> bool {
        self.options.dialect == Dialect::MiniScript
    }

    fn shorthand_assignment(&mut self, id: NodeId, variable: NodeId, init: NodeId) -> Result {
        let target = self.unwrap(variable);
        let NodeKind::Binary(binary) = self.kind(init) else {
            return self.assignment(id, variable, init);
        };
        let (operator, read, value) = (binary.operator, binary.left, binary.right);

        match self.kind(target) {
            NodeKind::Index(node) if !self.is_super(node.base) => {
                let call = match self.kind(read) {
                    NodeKind::Index(read) => read.is_statement_start.then_some(0),
                    _ => None,
                };

                self.expression(node.base)?;
                self.expression(node.index)?;
                self.emit(Op::Duplicate(2), target);
                self.emit(
                    Op::GetProperty {
                        call,
                        discard: false,
                    },
                    node.index,
                );
                self.expression_with_capture(value, true)?;
                self.emit(Op::binary(Operator::from(operator)), init);
                self.emit(Op::Assign, id);
            }
            NodeKind::Member(node) if !self.is_super(node.base) => {
                let key = self.key(self.name(node.identifier).unwrap_or_default());

                self.expression(node.base)?;
                self.emit(Op::Duplicate(1), target);
                self.emit(
                    Op::GetMember {
                        key: key.clone(),
                        call: Some(0),
                        discard: false,
                    },
                    node.identifier,
                );
                self.expression_with_capture(value, true)?;
                self.emit(Op::binary(Operator::from(operator)), init);
                self.emit(Op::AssignMember(key), id);
            }
            _ => return self.assignment(id, variable, init),
        }

        Ok(())
    }

    fn assignment_target(&mut self, variable: NodeId) -> Result<Op> {
        let mut target = self.unwrap(variable);

        if let NodeKind::Unary(node) = self.kind(target) {
            target = self.unwrap(node.argument);
        }

        let op = match self.kind(target) {
            NodeKind::Member(node) => {
                self.expression(node.base)?;

                Op::AssignMember(self.key(self.name(node.identifier).unwrap_or_default()))
            }
            NodeKind::Index(node) => {
                self.expression(node.base)?;
                self.expression(node.index)?;

                Op::Assign
            }
            NodeKind::Identifier(_) => self.assign_local(self.name(target).unwrap_or_default()),
            _ => {
                self.expression(target)?;
                self.push(Value::Null, target);

                Op::Assign
            }
        };

        Ok(op)
    }

    fn assign_local(&mut self, name: &str) -> Op {
        match self.slot(name) {
            Some(slot) => Op::AssignSlot {
                slot,
                value: Operand::Stack,
            },
            None => Op::AssignLocal {
                key: self.key(name),
                cache: EntryCache::default(),
            },
        }
    }

    fn return_statement(&mut self, id: NodeId, argument: Option<NodeId>) -> Result {
        if self.is_top_level() {
            return argument.map_or(Ok(()), |argument| self.statement(argument));
        }

        match argument {
            Some(argument) => self.expression(argument)?,
            None => self.push(Value::Null, id),
        }

        self.emit(Op::Return(None), id);

        Ok(())
    }

    fn while_loop(&mut self, condition: NodeId, body: &[NodeId]) -> Result {
        let start = self.label();
        let check = self.label();
        let end = self.label();

        self.jump(Op::Jump(0), check, condition);
        self.set_label(start);
        self.loops().push((check, end));
        self.body(body)?;
        self.loops().pop();
        self.set_label(check);
        self.expression(condition)?;

        let repeat = Op::JumpIf {
            target: 0,
            when: Truthy::SemiTrue,
            push_condition: false,
        };

        self.jump(repeat, start, condition);
        self.set_label(end);

        Ok(())
    }

    fn if_statement(&mut self, id: NodeId, clauses: &[NodeId]) -> Result {
        let end = self.label();

        for &clause in clauses {
            self.if_clause(id, clause, end)?;
        }

        self.set_label(end);

        Ok(())
    }

    fn if_clause(&mut self, id: NodeId, clause: NodeId, end: Label) -> Result {
        match self.kind(clause) {
            NodeKind::IfClause(node) => {
                let next = self.label();

                self.expression(node.condition)?;
                self.jump(Self::jump_if_false(), next, id);
                self.body(self.ast().list(node.body))?;
                self.jump(Op::Jump(0), end, id);
                self.set_label(next);

                Ok(())
            }
            NodeKind::ElseClause(node) => self.body(self.ast().list(node.body)),
            _ => Err(Box::new(self.unexpected_node_error(clause))),
        }
    }

    fn for_loop(
        &mut self,
        id: NodeId,
        variable: NodeId,
        sequence: NodeId,
        body: &[NodeId],
    ) -> Result {
        let name = self.name(variable).unwrap_or_default();
        let index = Self::loop_index(name);
        let next = self.label();
        let end = self.label();

        self.push(-1.0, id);

        let assign_index = self.assign_local(&index);

        self.emit(assign_index, id);
        self.expression(sequence)?;
        self.emit(Op::PushIterator, sequence);

        let start = self.label();

        self.jump(Op::Jump(0), next, sequence);
        self.set_label(start);
        self.loops().push((next, end));
        self.body(body)?;
        self.loops().pop();
        self.set_label(next);

        let step = match (self.slot(name), self.slot(&index)) {
            (Some(variable), Some(index)) => Op::NextSlot {
                variable,
                index,
                body: 0,
            },
            _ => Op::Next {
                variable: Local::new_box(&self.key(name)),
                index: Local::new_box(&self.key(&index)),
                body: 0,
            },
        };

        self.jump(step, start, sequence);
        self.set_label(end);
        self.emit(Op::PopIterator, sequence);

        Ok(())
    }
}
