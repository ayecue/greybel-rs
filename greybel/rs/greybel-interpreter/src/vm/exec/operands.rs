use super::super::Vm;
use crate::bytecode::{Destination, Operand, Variable, VariableDestination, VariableOperand};
use crate::value::Value;

pub(super) enum Input {
    Stack,
    Value(Value),
}

impl Vm {
    #[inline(always)]
    pub(super) fn slot_value(&self, slot: u32) -> Option<&Value> {
        match self.slot(slot) {
            Some(Value::Function(_)) | None => None,
            Some(value) => Some(value),
        }
    }

    #[inline(always)]
    pub(super) fn variable_value(&self, variable: &Variable) -> Option<&Value> {
        match self.variable(&variable.key, &variable.cache) {
            Some(Value::Function(_)) | None => None,
            Some(value) => Some(value),
        }
    }

    pub(super) fn input(&self, operand: &Operand) -> Option<Input> {
        match operand {
            Operand::Stack => Some(Input::Stack),
            Operand::Value(value) => Some(Input::Value(value.clone())),
            Operand::Slot(slot) => self.slot_value(*slot).cloned().map(Input::Value),
        }
    }

    pub(super) fn variable_input(&self, operand: &VariableOperand) -> Option<Input> {
        match operand {
            VariableOperand::Stack => Some(Input::Stack),
            VariableOperand::Value(value) => Some(Input::Value(value.clone())),
            VariableOperand::Variable(variable) => {
                self.variable_value(variable).cloned().map(Input::Value)
            }
        }
    }

    pub(super) fn take(&mut self, input: Input) -> Value {
        match input {
            Input::Stack => self.pop(),
            Input::Value(value) => value,
        }
    }

    #[inline(always)]
    pub(super) fn put(&mut self, to: Destination, value: Value) {
        match to {
            Destination::Stack => self.stack.push(value),
            Destination::Slot(slot) => *self.slot_mut(slot) = Some(value),
        }
    }

    #[inline(always)]
    pub(super) fn put_variable(&mut self, to: &VariableDestination, value: Value) {
        match to {
            VariableDestination::Stack => self.stack.push(value),
            VariableDestination::Local(local) => {
                self.store_local(&local.key, value, &local.cache);
            }
        }
    }

    #[cold]
    #[inline(never)]
    pub(super) fn fall_back(&mut self) {
        let frame = self.frame();
        let start = self.program.code(frame.code).fallback(frame.current);

        self.frame_mut().ip = start;
    }
}
