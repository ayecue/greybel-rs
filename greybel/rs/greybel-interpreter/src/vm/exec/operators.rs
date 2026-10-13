use std::rc::Rc;

use greybel_core::BinaryOp;

use super::super::index::IndexResolver;
use super::super::{ValueType, Vm};
use crate::bytecode::{
    CodeId, Destination, Operand, Operator, Truthy, UnaryOperator, VariableDestination,
    VariableOperand,
};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::{Closure, FunctionBody, Number, Param, Value};

impl Vm {
    #[inline(never)]
    pub(super) fn unary(&mut self, operator: UnaryOperator, discard: bool) {
        let value = self.pop();
        let result = match operator {
            UnaryOperator::Not => match value {
                Value::Number(number) => Value::Number(Number::new(1.0) - number.clamp_truth()),
                other => Value::from(!self.heap.is_truthy(&other)),
            },
            UnaryOperator::Negate => match value {
                Value::Number(number) => Value::Number(-number),
                Value::Null => Value::Number(Number::ZERO),
                _ => Value::Null,
            },
            UnaryOperator::New => self.instance_of(value),
            UnaryOperator::MiniScriptNew => match value {
                Value::Map(parent) => match self.scopes.types.kind_of(parent) {
                    Some(kind) if kind != ValueType::Map => {
                        self.raise(RuntimeError::from(RuntimeErrorKind::NewOfType(kind)));

                        Value::Null
                    }
                    _ => self.instance_of(Value::Map(parent)),
                },
                _ => {
                    self.raise(RuntimeError::from(RuntimeErrorKind::NewOfNonMap));

                    Value::Null
                }
            },
        };

        self.push_unless(discard, result);
    }

    fn instance_of(&mut self, parent: Value) -> Value {
        let object = self.heap.add_map();

        object.set_key(&mut self.heap, &self.keys.isa, parent);

        Value::Map(object)
    }

    #[inline(always)]
    pub(in crate::vm) fn operate(&mut self, operator: Operator, a: &Value, b: &Value) -> Value {
        match (operator, a, b) {
            (Operator::Binary(operator), Value::Number(x), Value::Number(y)) => {
                Value::apply_numbers(operator, *x, *y)
            }
            (Operator::Binary(operator), _, _) => self.apply_binary(operator, a, b),
            (other, _, _) => self.apply_operator(other, a, b),
        }
    }

    #[cold]
    #[inline(never)]
    fn apply_operator(&mut self, operator: Operator, a: &Value, b: &Value) -> Value {
        match operator {
            Operator::Binary(operator) => self.apply_binary(operator, a, b),
            Operator::Bitwise(operator) => a.bitwise(operator, b),
            Operator::Isa => match self.is_a(a, b) {
                Ok(isa) => isa.into(),
                Err(error) => {
                    self.raise(error.into());

                    Value::Null
                }
            },
            Operator::And => self.fuzzy_and(a, b),
            Operator::Or => self.fuzzy_or(a, b),
        }
    }

    #[inline(never)]
    pub(super) fn slice(&mut self) {
        let end = self.pop();
        let start = self.pop();
        let value = match self.pop() {
            Value::List(id) => {
                let items = id.items(&self.heap);
                let range = IndexResolver::new(items.len()).slice(&start, &end);
                let items = items[range].to_vec();

                Value::List(self.heap.add_list(items))
            }
            Value::String(string) => {
                let encoding = self.heap.string_encoding();
                let units = string.units(encoding);
                let range = IndexResolver::new(units.len()).slice(&start, &end);

                Value::String(units.slice(range))
            }
            _ => Value::Null,
        };

        self.stack.push(value);
    }

    pub(super) fn binary(&mut self, operator: Operator, right: Option<&Value>) {
        let result = match right {
            Some(right) => {
                let a = self.pop();

                self.operate(operator, &a, right)
            }
            None => {
                let b = self.pop();
                let a = self.pop();

                self.operate(operator, &a, &b)
            }
        };

        self.stack.push(result);
    }

    pub(super) fn slot_binary(
        &mut self,
        operator: Operator,
        left: &Operand,
        right: &Operand,
        to: Destination,
    ) {
        match self.evaluate(Produce, operator, left, right) {
            Some(result) => self.put(to, result),
            None => self.fall_back(),
        }
    }

    #[inline(always)]
    pub(super) fn evaluate<O: Outcome>(
        &mut self,
        outcome: O,
        operator: Operator,
        left: &Operand,
        right: &Operand,
    ) -> Option<O::Output> {
        match (left, right) {
            (Operand::Stack, Operand::Stack) => {
                let b = self.pop();
                let a = self.pop();

                Some(outcome.operate(self, operator, &a, &b))
            }
            (Operand::Stack, Operand::Value(b)) => {
                let a = self.pop();

                Some(outcome.operate(self, operator, &a, b))
            }
            (Operand::Slot(a), Operand::Value(b)) => {
                let a = self.slot_value(*a)?;

                if let (Value::Number(x), Value::Number(y), Operator::Binary(operator)) =
                    (a, b, operator)
                {
                    return Some(outcome.numbers(operator, *x, *y));
                }

                let a = a.clone();

                Some(outcome.operate(self, operator, &a, b))
            }
            (Operand::Slot(a), Operand::Slot(b)) => {
                let (a, b) = (self.slot_value(*a)?, self.slot_value(*b)?);

                if let (Value::Number(x), Value::Number(y), Operator::Binary(operator)) =
                    (a, b, operator)
                {
                    return Some(outcome.numbers(operator, *x, *y));
                }

                let (a, b) = (a.clone(), b.clone());

                Some(outcome.operate(self, operator, &a, &b))
            }
            (Operand::Stack, Operand::Slot(b)) => {
                let b = self.slot_value(*b)?.clone();
                let a = self.pop();

                Some(outcome.operate(self, operator, &a, &b))
            }
            _ => self.evaluate_any(outcome, operator, left, right),
        }
    }

    #[inline(never)]
    fn evaluate_any<O: Outcome>(
        &mut self,
        outcome: O,
        operator: Operator,
        left: &Operand,
        right: &Operand,
    ) -> Option<O::Output> {
        let b = self.input(right)?;
        let a = self.input(left)?;
        let b = self.take(b);
        let a = self.take(a);

        Some(outcome.operate(self, operator, &a, &b))
    }

    pub(super) fn variable_binary(
        &mut self,
        operator: Operator,
        left: &VariableOperand,
        right: &VariableOperand,
        to: &VariableDestination,
    ) {
        match self.evaluate_variables(Produce, operator, left, right) {
            Some(result) => self.put_variable(to, result),
            None => self.fall_back(),
        }
    }

    #[inline(always)]
    pub(super) fn evaluate_variables<O: Outcome>(
        &mut self,
        outcome: O,
        operator: Operator,
        left: &VariableOperand,
        right: &VariableOperand,
    ) -> Option<O::Output> {
        match (left, right) {
            (VariableOperand::Variable(a), VariableOperand::Value(b)) => {
                let a = self.variable_value(a)?;

                if let (Value::Number(x), Value::Number(y), Operator::Binary(operator)) =
                    (a, b, operator)
                {
                    return Some(outcome.numbers(operator, *x, *y));
                }

                let a = a.clone();

                Some(outcome.operate(self, operator, &a, b))
            }
            (VariableOperand::Variable(a), VariableOperand::Variable(b)) => {
                let (a, b) = (self.variable_value(a)?, self.variable_value(b)?);

                if let (Value::Number(x), Value::Number(y), Operator::Binary(operator)) =
                    (a, b, operator)
                {
                    return Some(outcome.numbers(operator, *x, *y));
                }

                let (a, b) = (a.clone(), b.clone());

                Some(outcome.operate(self, operator, &a, &b))
            }
            (VariableOperand::Stack, VariableOperand::Variable(b)) => {
                let b = self.variable_value(b)?.clone();
                let a = self.pop();

                Some(outcome.operate(self, operator, &a, &b))
            }
            _ => self.evaluate_variables_any(outcome, operator, left, right),
        }
    }

    #[inline(never)]
    fn evaluate_variables_any<O: Outcome>(
        &mut self,
        outcome: O,
        operator: Operator,
        left: &VariableOperand,
        right: &VariableOperand,
    ) -> Option<O::Output> {
        let b = self.variable_input(right)?;
        let a = self.variable_input(left)?;
        let b = self.take(b);
        let a = self.take(a);

        Some(outcome.operate(self, operator, &a, &b))
    }

    #[inline(never)]
    pub(super) fn compare_chain(&mut self, operators: &[BinaryOp]) {
        let count = u32::try_from(operators.len() + 1).expect("few comparisons");
        let values = self.pop_values(count);
        let all_true = operators
            .iter()
            .zip(values.windows(2))
            .all(|(&operator, pair)| {
                let result = self.apply_binary(operator, &pair[0], &pair[1]);

                self.heap.is_truthy(&result)
            });

        self.stack.push(all_true.into());
    }

    pub(super) fn make_list(&mut self, items: u32, discard: bool) {
        let items = self.pop_values(items);

        if !discard {
            let list = self.heap.add_list(items);

            self.stack.push(Value::List(list));
        }
    }

    #[inline(never)]
    pub(super) fn make_map(&mut self, entries: u32, discard: bool) {
        let at = self.stack.len().saturating_sub(entries as usize * 2);

        if discard {
            self.stack.truncate(at);

            return;
        }

        let map = self.heap.add_map_with_capacity(entries as usize);
        let mut values = self.stack.drain(at..);

        while let (Some(key), Some(value)) = (values.next(), values.next()) {
            map.set(&mut self.heap, key, value);
        }

        drop(values);
        self.stack.push(Value::Map(map));
    }

    pub(super) fn make_function(
        &mut self,
        params: &Rc<[Param]>,
        code: CodeId,
        capture_outer: bool,
    ) {
        let outer = if capture_outer {
            self.locals_map()
        } else {
            self.scopes.globals
        };
        let function = self.heap.add_function(Closure {
            params: Rc::clone(params),
            body: FunctionBody::Script(code),
            outer: Some(outer),
        });

        self.stack.push(Value::Function(function));
    }
}

pub(super) trait Outcome: Copy {
    type Output;

    fn numbers(self, operator: BinaryOp, x: Number, y: Number) -> Self::Output;
    fn operate(self, vm: &mut Vm, operator: Operator, a: &Value, b: &Value) -> Self::Output;
}

#[derive(Clone, Copy)]
pub(super) struct Produce;

impl Outcome for Produce {
    type Output = Value;

    #[inline(always)]
    fn numbers(self, operator: BinaryOp, x: Number, y: Number) -> Value {
        Value::apply_numbers(operator, x, y)
    }

    #[inline(always)]
    fn operate(self, vm: &mut Vm, operator: Operator, a: &Value, b: &Value) -> Value {
        vm.operate(operator, a, b)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Branch(pub(super) Truthy);

impl Outcome for Branch {
    type Output = bool;

    #[inline(always)]
    fn numbers(self, operator: BinaryOp, x: Number, y: Number) -> bool {
        match Value::compare_numbers(operator, x, y) {
            Some(truth) => self.0.matches_bool(truth),
            None => {
                let result = Value::apply_numbers(operator, x, y);

                Truthy::matches_number(Number::new(result.to_number()).clamp_truth(), self.0)
            }
        }
    }

    #[inline(always)]
    fn operate(self, vm: &mut Vm, operator: Operator, a: &Value, b: &Value) -> bool {
        if let (Operator::Binary(operator), Value::Number(x), Value::Number(y)) = (operator, a, b) {
            return self.numbers(operator, *x, *y);
        }

        let result = vm.operate(operator, a, b);

        Truthy::matches_number(vm.condition(&result), self.0)
    }
}
