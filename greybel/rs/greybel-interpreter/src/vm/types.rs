use crate::value::{Map, Value};

pub(super) struct ScopeTable {
    pub globals: Map,
    pub api: Map,
    pub types: TypeTable,
    pub exports: Vec<Option<Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    String,
    Number,
    List,
    Map,
    Function,
}

impl ValueType {
    pub const ALL: [Self; 5] = [
        Self::String,
        Self::Number,
        Self::List,
        Self::Map,
        Self::Function,
    ];
}

#[derive(Debug, Clone, Copy)]
pub struct TypeTable {
    pub string: Map,
    pub number: Map,
    pub list: Map,
    pub map: Map,
    pub function: Map,
}

impl TypeTable {
    #[must_use]
    pub fn get(&self, kind: ValueType) -> Map {
        match kind {
            ValueType::String => self.string,
            ValueType::Number => self.number,
            ValueType::List => self.list,
            ValueType::Map => self.map,
            ValueType::Function => self.function,
        }
    }

    #[must_use]
    pub fn kind_of(&self, map: Map) -> Option<ValueType> {
        ValueType::ALL
            .into_iter()
            .find(|&kind| self.get(kind) == map)
    }
}
