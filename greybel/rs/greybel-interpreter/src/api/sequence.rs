use std::fmt;

use super::IntoNativeResult;
use crate::error::RuntimeError;
use crate::value::{Heap, Number, Value};
use crate::vm::NativeReturn;

pub trait SequenceItem: Into<Value> {}

impl SequenceItem for f64 {}
impl SequenceItem for Number {}
impl SequenceItem for bool {}
impl SequenceItem for String {}
impl SequenceItem for &'static str {}

pub struct Sequence<I> {
    items: I,
    capacity: usize,
}

impl<I> Sequence<I>
where
    I: Iterator + Clone + 'static,
    I::Item: SequenceItem,
{
    pub fn new(items: I) -> Self {
        let capacity = items.size_hint().0;

        Self::with_capacity(items, capacity)
    }

    pub fn with_capacity(items: I, capacity: usize) -> Self {
        Self { items, capacity }
    }

    fn into_values(self) -> Vec<Value> {
        let mut values = Vec::with_capacity(self.capacity);

        values.extend(self.items.map(Into::into));

        values
    }
}

impl<I> IntoNativeResult for Sequence<I>
where
    I: Iterator + Clone + 'static,
    I::Item: SequenceItem,
{
    fn into_native_result(self, heap: &mut Heap) -> Result<Value, RuntimeError> {
        Ok(Value::List(heap.add_list(self.into_values())))
    }

    fn into_native_return(self, _heap: &mut Heap) -> Result<NativeReturn, RuntimeError> {
        Ok(NativeReturn::sequence(Box::new(self)))
    }
}

pub(crate) trait SequenceReturn {
    fn nth(&mut self, skip: usize) -> Option<Value>;

    fn clone_items(&self) -> Box<dyn SequenceReturn>;

    fn into_values(self: Box<Self>) -> Vec<Value>;
}

impl<I> SequenceReturn for Sequence<I>
where
    I: Iterator + Clone + 'static,
    I::Item: SequenceItem,
{
    fn nth(&mut self, skip: usize) -> Option<Value> {
        self.items.nth(skip).map(Into::into)
    }

    fn clone_items(&self) -> Box<dyn SequenceReturn> {
        Box::new(Self::with_capacity(self.items.clone(), self.capacity))
    }

    fn into_values(self: Box<Self>) -> Vec<Value> {
        Sequence::into_values(*self)
    }
}

pub(crate) struct Cursor {
    start: Box<dyn SequenceReturn>,
    remainder: Box<dyn SequenceReturn>,
    next: usize,
}

impl Cursor {
    pub fn new(items: Box<dyn SequenceReturn>) -> Self {
        Self {
            remainder: items.clone_items(),
            start: items,
            next: 0,
        }
    }

    pub fn get(&mut self, index: usize) -> Option<Value> {
        if index < self.next {
            self.remainder = self.start.clone_items();
            self.next = 0;
        }

        let item = self.remainder.nth(index - self.next);

        self.next = if item.is_some() {
            index + 1
        } else {
            usize::MAX
        };

        item
    }
}

impl Clone for Cursor {
    fn clone(&self) -> Self {
        Self {
            start: self.start.clone_items(),
            remainder: self.remainder.clone_items(),
            next: self.next,
        }
    }
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "Cursor debug output reports the next index without rendering borrowed sequence items"
)]
impl fmt::Debug for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cursor").field("next", &self.next).finish()
    }
}
