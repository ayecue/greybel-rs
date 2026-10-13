use std::ops::{Deref, DerefMut};

use super::{AllocationSize, Heap};
use crate::value::{List, Value};

impl Heap {
    pub fn add_list(&mut self, items: Vec<Value>) -> List {
        self.last_allocation_size += items.allocation_size();

        List(self.lists.insert(items))
    }
}

impl List {
    #[must_use]
    pub fn items(self, heap: &Heap) -> &[Value] {
        heap.lists.get(self.0)
    }

    pub fn items_mut(self, heap: &mut Heap) -> ItemsMut<'_> {
        let Heap {
            lists,
            last_allocation_size: allocation_size,
            ..
        } = heap;
        let items = lists.get_mut(self.0);

        ItemsMut {
            capacity: items.capacity(),
            items,
            allocation_size,
        }
    }
}

pub struct ItemsMut<'h> {
    items: &'h mut Vec<Value>,
    capacity: usize,
    allocation_size: &'h mut usize,
}

impl Deref for ItemsMut<'_> {
    type Target = Vec<Value>;

    fn deref(&self) -> &Vec<Value> {
        self.items
    }
}

impl DerefMut for ItemsMut<'_> {
    fn deref_mut(&mut self) -> &mut Vec<Value> {
        self.items
    }
}

impl Drop for ItemsMut<'_> {
    fn drop(&mut self) {
        let capacity_gain = self.items.capacity().saturating_sub(self.capacity);

        *self.allocation_size += capacity_gain * size_of::<Value>();
    }
}
