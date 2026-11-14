//! Typed collections of module capabilities.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::registry::ServiceHolder;

/// Typed capabilities contributed by any number of modules.
#[derive(Default)]
pub struct CapabilityRegistry {
    values: RefCell<HashMap<TypeId, Vec<Rc<dyn Any>>>>,
}

impl CapabilityRegistry {
    pub(crate) fn add<T: ?Sized + 'static>(&self, value: Rc<T>) {
        self.values
            .borrow_mut()
            .entry(TypeId::of::<T>())
            .or_default()
            .push(Rc::new(ServiceHolder(value)));
    }

    /// Returns all capabilities registered as `T`, in registration order.
    #[must_use]
    pub fn all<T: ?Sized + 'static>(&self) -> Vec<Rc<T>> {
        self.values
            .borrow()
            .get(&TypeId::of::<T>())
            .into_iter()
            .flatten()
            .map(|value| {
                value
                    .clone()
                    .downcast::<ServiceHolder<T>>()
                    .expect("capabilities are indexed by their registered type")
                    .0
                    .clone()
            })
            .collect()
    }
}
