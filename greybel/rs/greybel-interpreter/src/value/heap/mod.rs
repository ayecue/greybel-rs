use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

use greybel_core::PositionEncoding;

mod collect;
mod lists;
mod maps;
mod objects;

pub use self::collect::HeapRef;
pub use self::lists::ItemsMut;
use self::objects::ObjectTable;
use super::{Closure, Function, Map, Text, TextUnitView, Value, ValueMap};

pub type MapLabel = fn(&Heap, Map) -> Option<String>;

trait AllocationSize {
    fn allocation_size(&self) -> usize;
}

impl AllocationSize for Vec<Value> {
    fn allocation_size(&self) -> usize {
        size_of::<Self>() + self.capacity() * size_of::<Value>()
    }
}

impl AllocationSize for ValueMap {
    fn allocation_size(&self) -> usize {
        size_of::<Self>() + self.allocation_size()
    }
}

impl AllocationSize for Closure {
    fn allocation_size(&self) -> usize {
        size_of::<Self>()
    }
}

#[derive(Debug, Default)]
pub struct Heap {
    lists: ObjectTable<Vec<Value>>,
    maps: ObjectTable<ValueMap>,
    #[expect(
        clippy::box_collection,
        reason = "a larger heap shifts the VM's hot fields and costs 2% on map-heavy benchmarks"
    )]
    hosts: Option<Box<HashMap<u32, Rc<dyn Any>>>>,
    functions: ObjectTable<Closure>,
    last_allocation_size: usize,
    current_allocation_size: usize,
    spare_maps: Vec<ValueMap>,
    map_insertions: Vec<u32>,
    map_label: Option<MapLabel>,
    string_encoding: PositionEncoding,
}

impl Heap {
    #[must_use]
    pub fn with_map_label(mut self, label: MapLabel) -> Self {
        self.map_label = Some(label);

        self
    }

    #[must_use]
    pub fn map_label(&self) -> Option<MapLabel> {
        self.map_label
    }

    #[must_use]
    pub fn with_string_encoding(mut self, encoding: PositionEncoding) -> Self {
        self.string_encoding = encoding;

        self
    }

    #[must_use]
    pub fn string_encoding(&self) -> PositionEncoding {
        self.string_encoding
    }

    #[must_use]
    pub fn text_len(&self, text: &str) -> usize {
        self.string_encoding.count(text)
    }

    #[must_use]
    pub fn text_units<'t>(&self, text: &'t Text) -> TextUnitView<'t> {
        text.units(self.string_encoding)
    }

    pub fn add_function(&mut self, function: Closure) -> Function {
        self.last_allocation_size += function.allocation_size();

        Function(self.functions.insert(function))
    }

    #[must_use]
    pub fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Null => false,
            Value::Number(number) => number.is_truthy(),
            Value::String(text) => !text.is_empty(),
            Value::List(list) => !list.items(self).is_empty(),
            Value::Map(map) => !map.entries(self).is_empty(),
            Value::Function(_) => true,
        }
    }
}

impl Function {
    #[must_use]
    pub fn closure(self, heap: &Heap) -> &Closure {
        heap.functions.get(self.0)
    }
}
