use greybel_core::Range;
use greybel_documents::Uri;

use crate::ide::SymbolKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: SymbolKind,
    pub uri: Uri,
    pub range: Range,
    pub selection_range: Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyCall {
    pub item: CallHierarchyItem,
    pub ranges: Vec<Range>,
}
