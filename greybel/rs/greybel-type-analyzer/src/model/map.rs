use std::hash::{Hash, Hasher};

use greybel_symbols::names::ISA;
use indexmap::{Equivalent, IndexMap};
use meta_utils::BaseType;
use rustc_hash::FxBuildHasher;

use super::{CallContextId, DocumentIndex, GenericInstanceId, MapId, NameId, Site, SlotId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    String(Box<str>),
    Number(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyRef<'a> {
    String(&'a str),
    Number(u64),
}

impl Key {
    #[must_use]
    pub fn number(value: f64) -> Self {
        Self::Number(if value == 0.0 { 0 } else { value.to_bits() })
    }

    #[must_use]
    pub fn as_key_ref(&self) -> KeyRef<'_> {
        match self {
            Self::String(name) => KeyRef::String(name),
            Self::Number(bits) => KeyRef::Number(*bits),
        }
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_key_ref().hash(state);
    }
}

impl<'a> KeyRef<'a> {
    #[must_use]
    pub fn to_key(self) -> Key {
        match self {
            Self::String(name) => Key::String(name.into()),
            Self::Number(bits) => Key::Number(bits),
        }
    }

    #[must_use]
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            Self::String(name) => Some(name),
            Self::Number(_) => None,
        }
    }

    pub(crate) fn is_isa(self) -> bool {
        matches!(self, Self::String(name) if name == ISA)
    }
}

impl Equivalent<Key> for KeyRef<'_> {
    fn equivalent(&self, key: &Key) -> bool {
        *self == key.as_key_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapData {
    pub origin: MapOrigin,
    pub context: Option<CallContextId>,
    pub isa: SlotId,
    pub entries: IndexMap<Key, SlotId, FxBuildHasher>,
    pub dynamic_key: SlotId,
    pub dynamic_value: SlotId,
}

impl MapData {
    pub(crate) fn own_entry(&self, key: KeyRef<'_>) -> Option<SlotId> {
        if key.is_isa() {
            return Some(self.isa);
        }

        self.entries.get(&key).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapOrigin {
    Prototype(BaseType),
    Literal(Site),
    New(Site),
    Iteration(Site),
    Module(DocumentIndex),
    Custom(NameId),
    NestedKey { owner: MapId, name: NameId },
    GenericInstance(GenericInstanceId),
}
