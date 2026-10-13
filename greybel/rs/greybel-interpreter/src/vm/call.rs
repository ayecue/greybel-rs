use std::rc::Rc;

use greybel_symbols::names::SELF;

use super::frame::{CallStack, Frame, Iteration};
use super::native::{
    ArgContainer, AsyncNativeFn, NativeFn, NativeFuture, NativeReturn, NativeReturnKind,
};
use super::{State, Vm};
use crate::api::sequence::{Cursor, SequenceReturn};
use crate::bytecode::{Op, SlotLayout};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::{FunctionBody, Map, Param, Value};

pub(crate) struct CallContext {
    pub self_value: Option<Value>,
    pub super_value: Option<Value>,
}

impl CallContext {
    pub fn new(self_value: Option<Value>, super_value: Option<Value>) -> Self {
        Self {
            self_value,
            super_value,
        }
    }
}

impl Vm {
    pub(super) fn super_of(&self, origin: Option<Map>) -> Option<Value> {
        origin.map(|map| self.isa_of(map).map_or(Value::Null, Value::Map))
    }

    pub(super) fn invoke(
        &mut self,
        callee: Value,
        count: usize,
        ctx: Option<CallContext>,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        let Value::Function(id) = callee else {
            if count > 0 {
                return Err(RuntimeError::from(RuntimeErrorKind::TooManyArguments));
            }

            self.push_unless(discard, callee);

            return Ok(());
        };
        let function = id.closure(&self.heap);
        let outer = function.outer;
        let params = &function.params;
        let skip =
            usize::from(ctx.is_some() && params.first().is_some_and(|param| &*param.name == SELF));

        if count + skip > params.len() {
            return Err(RuntimeError::from(RuntimeErrorKind::TooManyArguments));
        }

        let self_value = self.call_self(count, params, ctx.as_ref());
        let code = match &function.body {
            FunctionBody::Script(code) => *code,
            FunctionBody::Native { call, label } => {
                let (call, label) = (Rc::clone(call), *label);
                let args = self.native_args(Rc::clone(params), count, skip, self_value);
                let result = self.call_native(&call, label, &args, discard);

                self.recycle(args);

                return result;
            }
            FunctionBody::AsyncNative { call, label } => {
                let (call, label) = (Rc::clone(call), *label);
                let args = self.native_args(Rc::clone(params), count, skip, self_value);

                self.call_async_native(&call, label, &args, discard);
                self.recycle(args);

                return Ok(());
            }
        };

        self.check_memory()?;

        let super_value = match ctx {
            Some(ctx) => ctx.super_value,
            None => self.super_of_self(self_value.as_ref()),
        };
        let outer = outer.or(self.frame().locals);

        if let Some(slots) = &self.program.code(code).slots {
            let params = &id.closure(&self.heap).params;
            let mut frame = Frame::new(code, None, outer, discard);
            let base = self
                .calls
                .bind_slots(&mut self.stack, slots, params, count, skip);

            frame.slot_base = Some(base);
            if let Some(slot) = slots.self_slot {
                self.calls.slots[base + slot as usize].clone_from(&self_value);
            }

            if let Some(slot) = slots.super_slot {
                self.calls.slots[base + slot as usize].clone_from(&super_value);
            }

            frame.self_value = self_value;
            frame.super_value = super_value;
            self.calls.frames.push(frame);

            return Ok(());
        }

        let params = Rc::clone(&id.closure(&self.heap).params);
        let locals = self.heap.add_map_with_capacity(params.len() + 2);
        let mut frame = Frame::new(code, Some(locals), outer, discard);

        self.bind_script(locals, &params, count, skip);
        if let Some(value) = &self_value {
            locals.set_key(&mut self.heap, &self.keys.self_key, value.clone());
        }

        if let Some(value) = &super_value {
            locals.set_key(&mut self.heap, &self.keys.super_key, value.clone());
        }

        frame.self_value = self_value;
        frame.super_value = super_value;
        self.calls.frames.push(frame);

        Ok(())
    }

    fn call_self(
        &self,
        count: usize,
        params: &[Param],
        ctx: Option<&CallContext>,
    ) -> Option<Value> {
        if let Some(ctx) = ctx {
            return ctx.self_value.clone();
        }

        let at = self.stack.len() - count;

        (0..count)
            .find(|&index| &*params[index].name == SELF)
            .map(|index| self.stack[at + index].clone())
    }

    fn super_of_self(&mut self, self_value: Option<&Value>) -> Option<Value> {
        match self_value {
            Some(Value::Map(map)) => Some(
                self.isa_of(*map)
                    .map_or_else(|| Value::Map(self.heap.add_map()), Value::Map),
            ),
            _ => None,
        }
    }

    fn bind_script(&mut self, locals: Map, params: &[Param], count: usize, skip: usize) {
        for index in (0..count).rev() {
            let argument = self.stack.pop().expect("the arguments are on the stack");
            let param = &params[index + skip];

            if &*param.name != SELF {
                locals.set_key(&mut self.heap, &param.key, argument);
            }
        }

        for param in &params[count + skip..] {
            locals.set_key(&mut self.heap, &param.key, param.default.clone());
        }
    }

    fn native_args(
        &mut self,
        params: Rc<[Param]>,
        count: usize,
        skip: usize,
        self_value: Option<Value>,
    ) -> ArgContainer {
        let self_value = self_value.unwrap_or_default();
        let mut values = self.spare_arguments.take().unwrap_or_default();

        values.extend(params.iter().map(|param| param.default.clone()));
        if skip == 1 {
            values[0] = self_value.clone();
        }

        for index in (0..count).rev() {
            values[index + skip] = self.stack.pop().expect("the arguments are on the stack");
        }

        ArgContainer {
            params,
            values,
            self_value,
        }
    }

    fn recycle(&mut self, args: ArgContainer) {
        let mut values = args.values;

        values.clear();

        match &self.spare_arguments {
            Some(spare) if spare.capacity() >= values.capacity() => {}
            _ => self.spare_arguments = Some(values),
        }
    }

    fn call_native(
        &mut self,
        native: &NativeFn,
        label: Option<&'static str>,
        args: &ArgContainer,
        discard: bool,
    ) -> Result<(), RuntimeError> {
        self.native_depth += 1;

        let result = native(self, args);

        self.native_depth -= 1;

        let outcome = match (result, label) {
            (Err(error), label) => match error.returned_message() {
                Some(message) => NativeReturn::from(Value::from(message)),
                None if let Some(label) = label => return Err(error.with_native_label(label)),
                None => return Err(error),
            },
            (Ok(outcome), _) => outcome,
        };

        match outcome.0 {
            NativeReturnKind::Value(value) => self.push_unless(discard, value),
            NativeReturnKind::Sequence(items) => self.return_sequence(items, discard),
            NativeReturnKind::Pending(future) => self.await_native(future, label, discard),
        }

        Ok(())
    }

    #[inline(never)]
    fn return_sequence(&mut self, items: Box<dyn SequenceReturn>, discard: bool) {
        if discard {
            return;
        }

        let frame = self.frame();
        let code = self.program.code(frame.code);

        if matches!(code.ops.get(frame.ip), Some(Op::PushIterator)) {
            let frame = self.frame_mut();

            frame.ip += 1;
            frame
                .iterations
                .push(Iteration::Sequence(Cursor::new(items)));

            return;
        }

        let list = self.heap.add_list(items.into_values());

        self.stack.push(Value::List(list));
    }

    fn call_async_native(
        &mut self,
        native: &AsyncNativeFn,
        label: Option<&'static str>,
        args: &ArgContainer,
        discard: bool,
    ) {
        self.native_depth += 1;

        let future = native(self, args);

        self.native_depth -= 1;
        self.await_native(future, label, discard);
    }

    fn await_native(&mut self, future: NativeFuture, label: Option<&'static str>, discard: bool) {
        self.pending_call = Some(match label {
            Some(label) => Box::pin(async move { future.await.with_label(label) }),
            None => future,
        });
        if self.state == State::Active {
            self.state = State::Pending { discard };
        }
    }
}

impl CallStack {
    fn bind_slots(
        &mut self,
        stack: &mut Vec<Value>,
        slots: &SlotLayout,
        params: &[Param],
        count: usize,
        skip: usize,
    ) -> usize {
        let base = self.slots.len();

        self.slots.resize(base + slots.names.len(), None);

        let frame = &mut self.slots[base..];

        for index in (skip..skip + count).rev() {
            let argument = stack.pop().expect("the arguments are on the stack");

            if &*params[index].name != SELF {
                frame[slots.params[index] as usize] = Some(argument);
            }
        }

        for index in skip + count..params.len() {
            frame[slots.params[index] as usize] = Some(params[index].default.clone());
        }

        base
    }
}
