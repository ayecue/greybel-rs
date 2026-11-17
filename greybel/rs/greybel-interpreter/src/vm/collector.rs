use std::mem::size_of;

use super::frame::{Frame, Iteration};
use super::{State, Vm};
use crate::error::{RuntimeError, RuntimeErrorKind};
use crate::value::{HeapRef, Value};

#[derive(Debug)]
pub struct Pin(u32);

pub(super) struct Collector {
    threshold: usize,
    pub next: usize,
    pins: Vec<Option<Value>>,
    memory_limit: usize,
}

impl Collector {
    pub fn new(threshold: usize, memory_limit: usize) -> Self {
        let threshold = threshold.max(1);

        Self {
            threshold,
            next: threshold,
            memory_limit,
            pins: Vec::new(),
        }
    }
}

impl Vm {
    pub fn pin(&mut self, value: Value) -> Pin {
        let vacant_index = self.collector.pins.iter().position(Option::is_none);
        let index = vacant_index.unwrap_or_else(|| {
            self.collector.pins.push(None);

            self.collector.pins.len() - 1
        });

        self.collector.pins[index] = Some(value);

        Pin(u32::try_from(index).expect("fewer than 2^32 pins"))
    }

    #[must_use]
    pub fn pinned_value(&self, pin: &Pin) -> &Value {
        self.collector.pins[pin.0 as usize]
            .as_ref()
            .expect("a pinned value")
    }

    pub fn unpin(&mut self, pin: &Pin) -> Value {
        self.collector.pins[pin.0 as usize]
            .take()
            .expect("a pinned value")
    }

    #[must_use]
    pub fn memory_usage(&self) -> usize {
        self.heap.current_allocation_size()
            + self.calls.frames.len() * size_of::<Frame>()
            + self.calls.slots.len() * size_of::<Option<Value>>()
            + self.stack.len() * size_of::<Value>()
    }

    pub(super) fn check_memory(&self) -> Result<(), RuntimeError> {
        if self.memory_usage() > self.collector.memory_limit {
            return Err(RuntimeError::from(RuntimeErrorKind::MemoryLimit));
        }

        Ok(())
    }

    pub fn collect_garbage(&mut self) -> usize {
        if self.native_depth > 0 || matches!(self.state, State::Pending { .. }) {
            return 0;
        }

        let roots = self.roots();
        let freed = self.heap.collect(roots);

        self.collector.next = self
            .collector
            .threshold
            .max(self.heap.current_allocation_size());

        freed
    }

    #[inline(never)]
    pub(crate) fn roots(&self) -> Vec<HeapRef> {
        let frames = self.calls.frames.iter().flat_map(frame_roots);
        let types = [
            self.scopes.types.string,
            self.scopes.types.number,
            self.scopes.types.list,
            self.scopes.types.map,
            self.scopes.types.function,
        ];
        let scopes = [self.scopes.globals, self.scopes.api]
            .into_iter()
            .chain(types)
            .map(HeapRef::Map);
        let values = self
            .stack
            .iter()
            .chain(self.calls.slots.iter().flatten())
            .chain(self.scopes.exports.iter().flatten())
            .chain(self.collector.pins.iter().flatten())
            .chain(self.native_functions.values());
        let mut roots: Vec<HeapRef> = values
            .filter_map(Value::heap_ref)
            .chain(frames)
            .chain(scopes)
            .collect();

        self.add_stack_roots(&mut roots);

        roots
    }
}

pub(super) fn frame_roots(frame: &Frame) -> impl Iterator<Item = HeapRef> + '_ {
    let scopes = [frame.locals, frame.outer]
        .into_iter()
        .flatten()
        .map(HeapRef::Map);
    let receiver = frame
        .self_value
        .iter()
        .chain(&frame.super_value)
        .filter_map(Value::heap_ref);
    let iterations = frame
        .iterations
        .iter()
        .filter_map(|iteration| match iteration {
            Iteration::List(id) => Some(HeapRef::List(*id)),
            Iteration::Map(id) => Some(HeapRef::Map(*id)),
            Iteration::Chars(_) | Iteration::Sequence(_) | Iteration::Empty => None,
        });

    scopes.chain(receiver).chain(iterations)
}
