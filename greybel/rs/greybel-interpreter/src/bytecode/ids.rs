#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CodeId(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImportSlot(pub(crate) u32);
