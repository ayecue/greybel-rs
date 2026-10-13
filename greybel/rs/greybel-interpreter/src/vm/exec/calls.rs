use super::super::{State, Vm};
use crate::error::RuntimeError;
use crate::value::Value;

impl Vm {
    pub(super) fn call(&mut self, arguments: u32, discard: bool) -> Result<(), RuntimeError> {
        let count = arguments as usize;
        let callee = self.take_below(count);

        self.invoke(callee, count, None, discard)
    }

    pub(super) fn return_from_frame(&mut self, value: Option<&Value>) {
        let value = match value {
            Some(value) => value.clone(),
            None => self.pop(),
        };

        if self.calls.frames.len() == 1 {
            self.state = State::Complete;
        } else if let Some(frame) = self.pop_frame() {
            self.push_unless(frame.discard, value);
        }
    }
}
