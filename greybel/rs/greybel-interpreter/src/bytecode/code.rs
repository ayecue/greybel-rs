use greybel_core::Span;

use super::{FileId, Op};
use crate::value::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    pub file: FileId,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct Code {
    pub ops: Vec<Op>,
    pub sources: Vec<Option<Source>>,
    pub slots: Option<SlotLayout>,
    pub fallbacks: Vec<(u32, u32)>,
}

impl Code {
    pub const END: u32 = u32::MAX;

    #[must_use]
    pub fn fallback(&self, index: usize) -> usize {
        let index = u32::try_from(index).expect("fewer than 2^32 instructions");
        let at = self
            .fallbacks
            .binary_search_by_key(&index, |&(op, _)| op)
            .expect("an op reading operands in place has a fallback");

        self.fallbacks[at].1 as usize
    }
}

#[derive(Debug, Clone)]
pub struct SlotLayout {
    pub names: Vec<Key>,
    pub params: Vec<u32>,
    pub self_slot: Option<u32>,
    pub super_slot: Option<u32>,
}
