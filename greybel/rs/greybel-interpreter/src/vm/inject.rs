use std::rc::Rc;

use super::frame::Frame;
use super::scheduler::Scheduler;
use super::{State, Vm};
use crate::bytecode::Program;
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::Map;

impl Vm {
    fn materialize_locals(&mut self) -> Map {
        let frame = self.frame();
        let Some(base) = frame.slot_base else {
            return self.locals_map();
        };
        let program = Rc::clone(&self.program);
        let slots = program
            .code(frame.code)
            .slots
            .as_ref()
            .expect("a slot layout");
        let locals = frame.locals.unwrap_or_else(|| self.heap.add_map());
        let frame = self.frame();
        let receiver = [
            (self.keys.self_key.clone(), frame.self_value.clone()),
            (self.keys.super_key.clone(), frame.super_value.clone()),
        ];

        for (name, value) in receiver {
            if let Some(value) = value {
                locals.set_key(&mut self.heap, &name, value);
            }
        }

        for (name, value) in slots.names.iter().zip(&self.calls.slots[base..]) {
            if let Some(value) = value {
                locals.set_key(&mut self.heap, name, value.clone());
            }
        }

        self.frame_mut().locals = Some(locals);

        locals
    }

    fn store_slots(&mut self, locals: Map) {
        let frame = self.frame();
        let Some(base) = frame.slot_base else { return };
        let program = Rc::clone(&self.program);
        let slots = program
            .code(frame.code)
            .slots
            .as_ref()
            .expect("a slot layout");

        for (offset, name) in slots.names.iter().enumerate() {
            if let Some(value) = locals.get_key(&self.heap, name) {
                self.calls.slots[base + offset] = Some(value.clone());
            }
        }
    }

    pub(crate) async fn inject(
        &mut self,
        program: Program,
        scheduler: &mut impl Scheduler,
    ) -> Result<(), RuntimeError> {
        let resume = self.state;

        if !matches!(resume, State::Active | State::Pause) {
            return Err(RuntimeError::from(RuntimeErrorKind::NotInjectable));
        }

        self.state = State::Active;

        let entry = Rc::make_mut(&mut self.program).append(program);
        let new_export_count = self.program.import_count as usize - self.scopes.exports.len();

        self.scopes
            .exports
            .extend(std::iter::repeat_n(None, new_export_count));

        let locals = self.materialize_locals();
        let current_frame = self.frame();
        let mut frame = Frame::new(entry, Some(locals), current_frame.outer, true);

        frame.self_value.clone_from(&current_frame.self_value);
        frame.super_value.clone_from(&current_frame.super_value);

        let depth = self.calls.frames.len();
        let stack = self.stack.len();
        let slots = self.calls.slots.len();
        let breakpoints = std::mem::replace(&mut self.breakpoints, false);
        let debug = self.take_debug_control();

        self.calls.frames.push(frame);

        let result = loop {
            if let Err(error) = self.await_pending_native().await {
                break Err(error);
            }

            if self.calls.frames.len() <= depth || self.state == State::Complete {
                break Ok(());
            }

            match self.run_batch(1) {
                Ok(_) if self.state == State::Failure => break Err(self.take_failure()),
                Ok(op_count) => {
                    if self.state == State::Yield {
                        self.state = State::Active;
                        self.yield_to(scheduler).await;
                    } else if scheduler.should_yield(op_count) {
                        self.yield_to(scheduler).await;
                    }
                }
                Err(error) => break Err(error),
            }
        };

        self.host.stdout.flush();
        self.breakpoints = breakpoints;
        self.restore_debug_control(debug);
        if result.is_err() {
            self.calls.slots.truncate(slots);
            self.calls.frames.truncate(depth);
            self.stack.truncate(stack);
        }

        if result.is_err() || self.state != State::Complete {
            self.state = resume;
        }

        self.store_slots(locals);

        result
    }
}
