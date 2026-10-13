use super::Heap;
use crate::value::{Function, List, Map, Value};

const MAX_SPARE_MAPS: usize = 4096;
const MAX_SPARE_CAPACITY: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapRef {
    List(List),
    Map(Map),
    Function(Function),
}

impl Value {
    #[must_use]
    pub fn heap_ref(&self) -> Option<HeapRef> {
        match self {
            Self::List(id) => Some(HeapRef::List(*id)),
            Self::Map(id) => Some(HeapRef::Map(*id)),
            Self::Function(id) => Some(HeapRef::Function(*id)),
            Self::Null | Self::Number(_) | Self::String(_) => None,
        }
    }
}

impl Heap {
    #[must_use]
    pub fn last_allocation_size(&self) -> usize {
        self.last_allocation_size
    }

    #[must_use]
    pub fn current_allocation_size(&self) -> usize {
        self.current_allocation_size
    }

    #[must_use]
    pub fn all_objects_len(&self) -> usize {
        self.lists.len() + self.maps.len() + self.functions.len()
    }

    pub fn collect(&mut self, roots: impl IntoIterator<Item = HeapRef>) -> usize {
        self.lists.unmark_all();
        self.maps.unmark_all();
        self.functions.unmark_all();

        let mut pending_refs: Vec<HeapRef> = roots.into_iter().collect();

        while let Some(object) = pending_refs.pop() {
            match object {
                HeapRef::List(id) => {
                    if self.lists.mark(id.0) {
                        pending_refs.extend(id.items(self).iter().filter_map(Value::heap_ref));
                    }
                }
                HeapRef::Map(id) => {
                    if self.maps.mark(id.0) {
                        for (key, item) in id.entries(self).iter() {
                            pending_refs.extend(key.heap_ref());
                            pending_refs.extend(item.heap_ref());
                        }
                    }
                }
                HeapRef::Function(id) => {
                    if self.functions.mark(id.0) {
                        let function = id.closure(self);

                        pending_refs.extend(function.outer.map(HeapRef::Map));
                        pending_refs.extend(
                            function
                                .params
                                .iter()
                                .filter_map(|param| param.default.heap_ref()),
                        );
                    }
                }
            }
        }

        self.last_allocation_size = 0;

        let spare = &mut self.spare_maps;
        let maps = self.maps.sweep(|mut map| {
            if spare.len() < MAX_SPARE_MAPS && map.capacity() <= MAX_SPARE_CAPACITY {
                map.clear();
                spare.push(map);
            }
        });

        if let Some(hosts) = &mut self.hosts {
            hosts.retain(|&map, _| self.maps.contains(map));
        }

        let swept = self.lists.sweep(drop) + maps + self.functions.sweep(drop);

        self.current_allocation_size = swept.current_allocation_size;

        swept.released_objects
    }
}
