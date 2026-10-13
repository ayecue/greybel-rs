use std::collections::BTreeMap;

use greybel_core::Range;
use greybel_documents::Uri;

use super::Location;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceEdit {
    pub changes: BTreeMap<Uri, Vec<TextEdit>>,
}

impl WorkspaceEdit {
    pub(crate) fn new(locations: Vec<Location>, text: &str) -> Self {
        let mut edit = Self::default();

        for location in locations {
            edit.changes
                .entry(location.uri)
                .or_default()
                .push(TextEdit {
                    range: location.range,
                    new_text: text.to_owned(),
                });
        }

        edit
    }
}
