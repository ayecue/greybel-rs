use greybel_core::{Position, Range};
use greybel_type_analyzer::{Analyzer, CallHierarchyCall, Dialect};

use super::{PATH, uri, with_document};

pub const SOURCE: &str = "helper = function(x)
  return x + 1
end function
run = function()
  a = helper(1)
  b = helper(2)
  f = @helper
  print a + b
end function
run
helper(3)";

/// `(name, call site starts)` of each call.
pub fn summary(calls: Vec<CallHierarchyCall>) -> Vec<(String, Vec<(u32, u32)>)> {
    calls
        .into_iter()
        .map(|call| {
            let starts = call
                .ranges
                .iter()
                .map(|range: &Range| (range.start.line, range.start.character))
                .collect();

            (call.item.name, starts)
        })
        .collect()
}

pub fn incoming(position: Position) -> Vec<(String, Vec<(u32, u32)>)> {
    let workspace = with_document(Dialect::Greybel, SOURCE);

    summary(Analyzer::new(&workspace).incoming_calls(&uri(PATH), position))
}

/// Callers of the function at `position` in `source`.
pub fn list_callers_in(source: &str, position: Position) -> Vec<(String, Vec<(u32, u32)>)> {
    let workspace = with_document(Dialect::Greybel, source);

    summary(Analyzer::new(&workspace).incoming_calls(&uri(PATH), position))
}

pub fn outgoing(position: Position) -> Vec<(String, Vec<(u32, u32)>)> {
    let workspace = with_document(Dialect::Greybel, SOURCE);

    summary(Analyzer::new(&workspace).outgoing_calls(&uri(PATH), position))
}

/// `(name, selection start)` of the items at `position`.
pub fn prepared(position: Position) -> Vec<(String, (u32, u32))> {
    let workspace = with_document(Dialect::Greybel, SOURCE);

    Analyzer::new(&workspace)
        .prepare_call_hierarchy(&uri(PATH), position)
        .into_iter()
        .map(|item| {
            let start = item.selection_range.start;

            (item.name, (start.line, start.character))
        })
        .collect()
}
