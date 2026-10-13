use std::ops::Deref;

use super::SlotId;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum SlotList {
    #[default]
    Empty,
    One(SlotId),
    Many(Vec<SlotId>),
}

impl SlotList {
    pub(crate) fn add(&mut self, slot: SlotId) {
        match self {
            Self::Empty => *self = Self::One(slot),
            Self::One(first) if *first != slot => *self = Self::Many(vec![*first, slot]),
            Self::Many(slots) if !slots.contains(&slot) => slots.push(slot),
            _ => {}
        }
    }

    pub(crate) fn extend(&mut self, slots: &[SlotId]) {
        for &slot in slots {
            self.add(slot);
        }
    }

    pub(crate) fn sort_by_id(mut self) -> Self {
        if let Self::Many(slots) = &mut self {
            slots.sort_unstable();
        }

        self
    }
}

impl Deref for SlotList {
    type Target = [SlotId];

    fn deref(&self) -> &[SlotId] {
        match self {
            Self::Empty => &[],
            Self::One(slot) => std::slice::from_ref(slot),
            Self::Many(slots) => slots,
        }
    }
}
