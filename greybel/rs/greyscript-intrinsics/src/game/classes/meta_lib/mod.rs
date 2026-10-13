//! `metaLib` objects for computer library files and router `kernel_router.so`.

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::protocol::ObjectId;

mod debug_tools;
mod is_patched;
mod lib_name;
mod overflow;
mod version;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("lib_name", &lib_name::lib_name::NATIVE),
    ("version", &version::version::NATIVE),
    ("debug_tools", &debug_tools::debug_tools::NATIVE),
    ("is_patched", &is_patched::is_patched::NATIVE),
    ("overflow", &overflow::overflow::NATIVE),
];

/// Returns the server identity of `metaLib` `this`.
fn library(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::MetaLib>(vm, this).map(|library| library.reference())
}
