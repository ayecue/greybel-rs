//! Module registration contract.

use crate::Registry;

/// A domain module built by the composition root.
pub trait Module: 'static {
    /// Key of the module's schema in `schema_versions`.
    const NAME: &'static str;

    /// Builds this module's services from lower modules and registers its capabilities.
    fn register(registry: &std::rc::Rc<Registry>) -> Self;
}
