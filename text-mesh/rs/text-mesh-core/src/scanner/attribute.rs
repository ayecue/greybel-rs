use crate::hash::{NameHash, ValueHash};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ValueKind {
    #[default]
    None,
    Number,
    Color,
    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Unit {
    #[default]
    Pixels,
    FontUnits,
    Percentage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Attribute {
    pub(crate) name: NameHash,
    pub(crate) value: ValueHash,
    pub(crate) kind: ValueKind,
    pub(crate) unit: Unit,
    pub(crate) start: usize,
    pub(crate) length: usize,
}
