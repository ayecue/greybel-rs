use super::Vm;
use crate::api::sequence::Cursor;
use crate::bytecode::{CodeId, Location};
use crate::value::{EncodedText, List, Map, Value};

#[derive(Debug, Clone)]
pub(crate) enum Iteration {
    List(List),
    Map(Map),
    Chars(EncodedText),
    Sequence(Cursor),
    Empty,
}

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    pub code: CodeId,
    pub ip: usize,
    pub current: usize,
    pub locals: Option<Map>,
    pub slot_base: Option<usize>,
    pub outer: Option<Map>,
    pub self_value: Option<Value>,
    pub super_value: Option<Value>,
    pub iterations: Vec<Iteration>,
    pub discard: bool,
}

impl Frame {
    pub fn new(code: CodeId, locals: Option<Map>, outer: Option<Map>, discard: bool) -> Self {
        Self {
            code,
            ip: 0,
            current: 0,
            locals,
            slot_base: None,
            outer,
            self_value: None,
            super_value: None,
            iterations: Vec::new(),
            discard,
        }
    }
}

pub(crate) struct CallStack {
    pub frames: Vec<Frame>,
    pub slots: Vec<Option<Value>>,
}

impl CallStack {
    pub fn new(entry: Frame) -> Self {
        Self {
            frames: vec![entry],
            slots: Vec::new(),
        }
    }
}

impl Vm {
    #[must_use]
    pub fn stack_trace(&self) -> Vec<Location> {
        self.all_frames()
            .filter_map(|(frame, _)| self.frame_location(frame))
            .collect()
    }

    pub(super) fn frame_location(&self, frame: &Frame) -> Option<Location> {
        let source = self.program.code(frame.code).sources.get(frame.current)?;

        source.map(|source| self.program.location(source))
    }

    pub(super) fn frame(&self) -> &Frame {
        self.calls.frames.last().expect("the entry frame stays")
    }

    pub(super) fn frame_mut(&mut self) -> &mut Frame {
        self.calls.frames.last_mut().expect("the entry frame stays")
    }

    pub(super) fn locals_map(&self) -> Map {
        self.frame()
            .locals
            .expect("only a body without slots reads its locals as a map")
    }

    pub(super) fn slot(&self, slot: u32) -> Option<&Value> {
        let base = self.frame().slot_base.expect("a body with slots");

        self.calls.slots[base + slot as usize].as_ref()
    }

    pub(super) fn slot_mut(&mut self, slot: u32) -> &mut Option<Value> {
        let base = self.frame().slot_base.expect("a body with slots");

        &mut self.calls.slots[base + slot as usize]
    }

    #[inline(always)]
    pub(super) fn pop_frame(&mut self) -> Option<Frame> {
        let frame = self.calls.frames.pop()?;

        if let Some(base) = frame.slot_base {
            self.calls.slots.truncate(base);
        }

        Some(frame)
    }
}
