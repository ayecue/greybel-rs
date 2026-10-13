//! Debug-library object support and patch inspection.

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::receiver::{self, receiver};
use crate::runtime::Handle;

mod apply_patch;
mod payload;
mod scan_debuglib;
mod unit_testing;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("scan", &scan_debuglib::scan_debuglib::NATIVE),
    ("apply_patch", &apply_patch::apply_patch::NATIVE),
    ("unit_testing", &unit_testing::unit_testing::NATIVE),
    ("payload", &payload::payload::NATIVE),
];

/// Returns the debug-library helper behind a game map when the map has that type.
fn reference<'v>(vm: &'v Vm, this: &Value) -> Option<&'v Handle> {
    receiver::<receiver::DebugLibrary>(vm, this)
}
