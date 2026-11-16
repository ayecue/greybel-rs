use greybel_core::Range;
use greybel_documents::Uri;

use super::SymbolKind;

/// A function, or the top level of a file for calls outside functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: Uri,
    pub range: Range,
    pub selection_range: Range,
}

/// The calls between two items: `ranges` are the call sites, in the caller's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyCall {
    pub item: CallHierarchyItem,
    pub ranges: Vec<Range>,
}
