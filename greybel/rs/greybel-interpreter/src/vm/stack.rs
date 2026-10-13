use super::Vm;
use crate::value::Value;

impl Vm {
    #[must_use]
    pub fn open_values(&self) -> usize {
        self.stack.len()
    }

    pub(super) fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or_default()
    }

    pub(super) fn pop_values(&mut self, count: u32) -> Vec<Value> {
        let at = self.stack.len().saturating_sub(count as usize);

        self.stack.split_off(at)
    }

    pub(super) fn duplicate(&mut self, count: usize) {
        let at = self.stack.len().saturating_sub(count);

        self.stack.extend_from_within(at..);
    }

    pub(super) fn take_below(&mut self, count: usize) -> Value {
        match self.stack.len().checked_sub(count + 1) {
            Some(at) => self.stack.remove(at),
            None => Value::Null,
        }
    }

    pub(super) fn push_unless(&mut self, discard: bool, value: Value) {
        if !discard {
            self.stack.push(value);
        }
    }
}
