//! Helpers for `if` branches that assign narrowed `item` values.

use greybel_type_analyzer::{Analyzer, Dialect};

use super::{PATH, locals, miniscript, uri, with_document};

pub const ITEM: &str = "item = 1\nif rnd then item = \"hello\"\n";
pub const THREE: &str = "item = 1\nif rnd then item = \"hello\"\nif rnd then item = {}\n";
pub const NULLABLE: &str = "item = null\nif rnd then item = \"hello\"\n";
pub const SHELL: &str = "m = {\"shell\": null}\nif rnd then m.shell = \"s\"\n";

/// Returns `item`'s displayed types in the `then` and `else` branches of `condition`.
pub fn branches(prelude: &str, condition: &str) -> Vec<String> {
    miniscript(
        &format!(
            "{prelude}if {condition} then\n  positive = item\nelse\n  negative = item\nend if"
        ),
        &["positive", "negative"],
    )
}

/// Returns the displayed types of `names`, locals of the `index`th function of a GreyScript `source`.
pub fn function_locals(source: &str, index: usize, names: &[&str]) -> Vec<String> {
    let workspace = with_document(Dialect::GreyScript, source);
    let analysis = Analyzer::new(&workspace)
        .analysis(&uri(PATH))
        .expect("the document is loaded");

    locals(analysis, index, names)
}
