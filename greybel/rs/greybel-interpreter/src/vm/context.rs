use std::any::{Any, TypeId};
use std::mem;

use greybel_symbols::names::{IS_GREYBEL, PARAMS};
use rustc_hash::FxHashMap;

use super::frame::{CallStack, Frame};
use super::{NativeFuture, State, Vm};
use crate::bytecode::{CodeId, Program};
use crate::error::RuntimeError;
use crate::value::{HeapRef, Map, Value};

pub(super) type HostData = FxHashMap<TypeId, Box<dyn Any>>;

pub type ContextEndFn = Box<dyn FnOnce(&mut Vm, ContextExit) -> NativeFuture>;

#[derive(Debug)]
pub enum ContextExit {
    Completion,
    Failure(RuntimeError),
    Termination,
}

pub struct ContextEntry {
    address: CodeId,
    params: Vec<String>,
    data: HostData,
    on_end: ContextEndFn,
}

impl ContextEntry {
    #[must_use]
    pub fn new(
        address: CodeId,
        params: Vec<String>,
        on_end: impl FnOnce(&mut Vm, ContextExit) -> NativeFuture + 'static,
    ) -> Self {
        Self {
            address,
            params,
            data: HostData::default(),
            on_end: Box::new(on_end),
        }
    }

    #[must_use]
    pub fn with_host_data<T: 'static>(mut self, data: T) -> Self {
        self.data.insert(TypeId::of::<T>(), Box::new(data));

        self
    }
}

pub(super) struct Context {
    stack: Vec<Value>,
    pub(super) calls: CallStack,
    pub(super) globals: Map,
    pub(super) exports: Vec<Option<Value>>,
    native_depth: u32,
    data: HostData,
    /// Controls whether the child-starting native call's result is discarded.
    discard: bool,
    on_child_end: ContextEndFn,
}

#[derive(Default)]
pub(super) struct ContextStack {
    pub(super) contexts: Vec<Context>,
    pub(super) data: HostData,
    pub(super) stack_frame_count: usize,
}

impl Vm {
    pub fn add_program(&mut self, program: Program) -> CodeId {
        let entry = std::rc::Rc::make_mut(&mut self.program).append(program);

        self.evaluate_exports();

        entry
    }

    #[must_use]
    pub fn context_depth(&self) -> usize {
        self.host.context_stack.contexts.len()
    }

    pub fn exit_contexts_n(&mut self, depth: usize) {
        if depth > self.context_depth() {
            return;
        }

        loop {
            self.pending_call = None;

            if !self.has_parent() {
                self.state = State::Complete;

                return;
            }

            let lowest = self.context_depth() == depth;

            self.pop_context(ContextExit::Termination);

            if lowest {
                return;
            }
        }
    }

    pub(super) fn push_context(&mut self, entry: ContextEntry, discard: bool) {
        let params = entry.params.into_iter().map(Value::from).collect();
        let params = Value::List(self.heap.add_list(params));
        let globals = self
            .heap
            .add_map_from([(IS_GREYBEL.into(), Value::TRUE), (PARAMS.into(), params)]);
        let frame = Frame::new(entry.address, Some(globals), None, false);
        let exports = std::iter::repeat_n(None, self.program.import_count as usize).collect();
        let context_stack = &mut self.host.context_stack;
        let parent = Context {
            stack: mem::take(&mut self.stack),
            calls: mem::replace(&mut self.calls, CallStack::new(frame)),
            globals: mem::replace(&mut self.scopes.globals, globals),
            exports: mem::replace(&mut self.scopes.exports, exports),
            native_depth: mem::take(&mut self.native_depth),
            data: mem::replace(&mut context_stack.data, entry.data),
            discard,
            on_child_end: entry.on_end,
        };

        context_stack.stack_frame_count += parent.calls.frames.len();
        context_stack.contexts.push(parent);
        self.pending_call = None;
        self.failure = None;
        self.state = State::Active;
    }

    pub(super) fn pop_context(&mut self, exit: ContextExit) {
        let context_stack = &mut self.host.context_stack;
        let parent = context_stack
            .contexts
            .pop()
            .expect("the running context has a parent");

        context_stack.stack_frame_count -= parent.calls.frames.len();
        context_stack.data = parent.data;
        self.stack = parent.stack;
        self.calls = parent.calls;
        self.scopes.globals = parent.globals;
        self.scopes.exports = parent.exports;
        self.native_depth = parent.native_depth;
        self.failure = None;
        self.evaluate_exports();

        let end = (parent.on_child_end)(self, exit);

        self.pending_call = Some(end);
        self.state = State::Pending {
            discard: parent.discard,
        };
    }

    pub(super) fn has_parent(&self) -> bool {
        !self.host.context_stack.contexts.is_empty()
    }

    fn evaluate_exports(&mut self) {
        let count = self.program.import_count as usize;
        let new_export_count = count.saturating_sub(self.scopes.exports.len());

        // Avoid `resize` and `vec![None; n]`: their extra `Vec<Option<Value>>::extend_with` calls prevent inlining into
        // the call path's slot binding.
        self.scopes
            .exports
            .extend(std::iter::repeat_n(None, new_export_count));
    }

    pub(super) fn add_stack_roots(&self, roots: &mut Vec<HeapRef>) {
        for context in &self.host.context_stack.contexts {
            let values = context
                .stack
                .iter()
                .chain(context.calls.slots.iter().flatten())
                .chain(context.exports.iter().flatten());

            roots.extend(values.filter_map(Value::heap_ref));
            roots.extend(
                context
                    .calls
                    .frames
                    .iter()
                    .flat_map(super::collector::frame_roots),
            );
            roots.push(HeapRef::Map(context.globals));
        }
    }

    pub(super) fn all_frames(&self) -> impl Iterator<Item = (&Frame, &CallStack)> {
        let current = self
            .calls
            .frames
            .iter()
            .rev()
            .map(|frame| (frame, &self.calls));
        let below = self
            .host
            .context_stack
            .contexts
            .iter()
            .rev()
            .flat_map(|context| {
                context
                    .calls
                    .frames
                    .iter()
                    .rev()
                    .map(move |frame| (frame, &context.calls))
            });

        current.chain(below)
    }
}
