use greybel_core::Range;
use greybel_documents::Uri;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Location {
    pub uri: Uri,
    pub range: Range,
}
