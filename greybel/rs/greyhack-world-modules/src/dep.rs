//! Lazy typed service dependencies.

use std::any::type_name;
use std::cell::OnceCell;
use std::ops::Deref;
use std::rc::{Rc, Weak};

use crate::Registry;

/// A service dependency resolved on first use and cached afterwards.
pub struct Dep<T: ?Sized + 'static> {
    pub(crate) registry: Weak<Registry>,
    pub(crate) service: OnceCell<Rc<T>>,
}

impl<T: ?Sized + 'static> Clone for Dep<T> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            service: self.service.clone(),
        }
    }
}

impl<T: ?Sized + 'static> Deref for Dep<T> {
    type Target = T;

    /// Resolves the service on first use.
    ///
    /// # Panics
    ///
    /// Panics with the service type's name if it was never registered.
    fn deref(&self) -> &Self::Target {
        self.service
            .get_or_init(|| {
                let registry = self
                    .registry
                    .upgrade()
                    .expect("the assembled world retains its service registry");
                let name = type_name::<T>();

                registry
                    .resolve::<T>()
                    .unwrap_or_else(|| panic!("{name} is not registered"))
            })
            .as_ref()
    }
}

#[expect(
    clippy::missing_fields_in_debug,
    reason = "Dependency debug output omits the registry weak reference"
)]
impl<T: ?Sized + 'static> std::fmt::Debug for Dep<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Dep")
            .field("type", &type_name::<T>())
            .field("service", &self.service.get().is_some())
            .finish()
    }
}
