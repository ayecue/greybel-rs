use super::super::Vm;
use super::super::frame::Frame;
use super::operators::{Branch, Outcome};
use crate::bytecode::{CodeId, ImportSlot, Local, Operand, Operator, Truthy, VariableOperand};
use crate::value::{Number, Value};

impl Vm {
    pub(super) fn jump(&mut self, target: u32) {
        self.frame_mut().ip = target as usize;
    }

    pub(super) fn jump_if(&mut self, target: u32, truthy: Truthy, push_condition: bool) {
        let value = self.pop();
        let condition = self.condition(&value);

        if push_condition {
            self.stack.push(Value::Number(condition));
        }

        if Truthy::matches_number(condition, truthy) {
            self.jump(target);
        }
    }

    pub(super) fn binary_jump_if(
        &mut self,
        operator: Operator,
        right: Option<&Value>,
        target: u32,
        truthy: Truthy,
    ) {
        let branch = Branch(truthy);
        let jumps = match right {
            Some(right) => {
                let a = self.pop();

                branch.operate(self, operator, &a, right)
            }
            None => {
                let b = self.pop();
                let a = self.pop();

                branch.operate(self, operator, &a, &b)
            }
        };

        if jumps {
            self.jump(target);
        }
    }

    pub(super) fn slot_binary_jump_if(
        &mut self,
        operator: Operator,
        left: &Operand,
        right: &Operand,
        target: u32,
        truthy: Truthy,
    ) {
        let branch = Branch(truthy);
        let numbers = match (left, right) {
            (Operand::Slot(a), Operand::Value(Value::Number(y))) => match self.slot_value(*a) {
                Some(Value::Number(x)) => Some((*x, *y)),
                _ => None,
            },
            (Operand::Slot(a), Operand::Slot(b)) => {
                match (self.slot_value(*a), self.slot_value(*b)) {
                    (Some(Value::Number(x)), Some(Value::Number(y))) => Some((*x, *y)),
                    _ => None,
                }
            }
            _ => None,
        };
        let jumps = match (operator, numbers) {
            (Operator::Binary(binary), Some((x, y))) => branch.numbers(binary, x, y),
            _ => return self.slot_branch(branch, operator, left, right, target),
        };

        if jumps {
            self.jump(target);
        }
    }

    #[inline(never)]
    fn slot_branch(
        &mut self,
        branch: Branch,
        operator: Operator,
        left: &Operand,
        right: &Operand,
        target: u32,
    ) {
        match self.evaluate(branch, operator, left, right) {
            Some(true) => self.jump(target),
            Some(false) => {}
            None => self.fall_back(),
        }
    }

    pub(super) fn variable_binary_jump_if(
        &mut self,
        operator: Operator,
        left: &VariableOperand,
        right: &VariableOperand,
        target: u32,
        truthy: Truthy,
    ) {
        match self.evaluate_variables(Branch(truthy), operator, left, right) {
            Some(true) => self.jump(target),
            Some(false) => {}
            None => self.fall_back(),
        }
    }

    #[inline(never)]
    pub(super) fn push_iterator(&mut self) {
        let value = self.pop();
        let iteration = self.iterator(&value);

        self.frame_mut().iterations.push(iteration);
    }

    pub(super) fn next_item(&mut self, variable: &Local, index: &Local, body: u32) {
        let current = match self.local(index) {
            Some(current) => current.to_number(),
            None => match self.lookup_loop_index(index) {
                Some(current) => current,
                None => return,
            },
        };
        let next = Number::new(current) + Number::ONE;

        self.store_local(&index.key, Value::Number(next), &index.cache);
        if let Some(item) = self.iterator_value(next) {
            self.store_local(&variable.key, item, &variable.cache);
            self.jump(body);
        }
    }

    #[cold]
    #[inline(never)]
    fn lookup_loop_index(&mut self, index: &Local) -> Option<f64> {
        match self.lookup(&index.key) {
            Ok(current) => Some(current.to_number()),
            Err(error) => {
                self.raise(error);

                None
            }
        }
    }

    pub(super) fn next_slot_item(&mut self, variable: u32, index: u32, body: u32) {
        let current = self.slot_mut(index).clone().unwrap_or_default();
        let next = Number::new(current.to_number()) + Number::ONE;

        *self.slot_mut(index) = Some(Value::Number(next));
        if let Some(item) = self.iterator_value(next) {
            *self.slot_mut(variable) = Some(item);
            self.jump(body);
        }
    }

    #[inline(never)]
    pub(super) fn import(&mut self, slot: ImportSlot, code: CodeId) {
        match &self.scopes.exports[slot.0 as usize] {
            Some(exports) => self.stack.push(exports.clone()),
            None => {
                let caller = self.frame().locals;
                let locals = self.heap.add_map();

                self.calls
                    .frames
                    .push(Frame::new(code, Some(locals), caller, false));
            }
        }
    }

    #[inline(never)]
    pub(super) fn export(&mut self, slot: ImportSlot) {
        if self.scopes.exports[slot.0 as usize].is_none() {
            let exports = self.pop();

            self.scopes.exports[slot.0 as usize] = Some(exports.clone());
            self.stack.push(exports);
        }
    }
}
