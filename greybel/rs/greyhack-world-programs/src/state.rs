use std::any::{Any, TypeId};
use std::collections::HashMap;

/// One default-created value per state type.
#[derive(Default)]
pub struct StateMap {
    values: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl std::fmt::Debug for StateMap {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StateMap")
            .field("len", &self.values.len())
            .finish_non_exhaustive()
    }
}

impl StateMap {
    /// Returns the state of `T` when it has been created.
    pub fn get<T: Send + 'static>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Returns the state of `T`, creating its default value on first access.
    pub fn state<T: Default + Send + 'static>(&mut self) -> &T {
        self.values
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<T>::default())
            .downcast_ref::<T>()
            .expect("the state map indexes values by their concrete type")
    }

    /// Returns mutable state of `T`, creating its default value on first access.
    pub fn state_mut<T: Default + Send + 'static>(&mut self) -> &mut T {
        self.values
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::<T>::default())
            .downcast_mut::<T>()
            .expect("the state map indexes values by their concrete type")
    }
}
