//! Documentation-comment fixtures.

use std::fmt::Display;

/// Returns the rendered form of `ty` when present, or `None` otherwise.
pub fn written<T: Display>(ty: &Option<T>) -> Option<String> {
    ty.as_ref().map(ToString::to_string)
}
