//! Helpers for `if` branches that assign narrowed `item` values.

use super::miniscript;

pub const ITEM: &str = "item = 1\nitem = \"hello\"\n";
pub const THREE: &str = "item = 1\nitem = \"hello\"\nitem = {}\n";

/// Returns `item`'s displayed types in the `then` and `else` branches of `condition`.
pub fn branches(prelude: &str, condition: &str) -> Vec<String> {
    miniscript(
        &format!(
            "{prelude}if {condition} then\n  positive = item\nelse\n  negative = item\nend if"
        ),
        &["positive", "negative"],
    )
}
