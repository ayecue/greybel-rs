/// Dense declaration-order field ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub(crate) u32);

/// Dense declaration-order union variant ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariantId(pub(crate) u32);

/// Dense declaration-order enumeration member ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemberId(pub(crate) u32);

impl FieldId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

impl VariantId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

impl MemberId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}
