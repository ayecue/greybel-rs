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
    pub(crate) fn of_file(uri: &Uri, edits: Vec<TextEdit>) -> Self {
        let mut edit = Self::default();

        edit.changes.insert(uri.clone(), edits);

        edit
    }

    pub(crate) fn add(&mut self, location: Location, text: String) {
        self.changes
            .entry(location.uri)
            .or_default()
            .push(TextEdit {
                range: location.range,
                new_text: text,
            });
    }
}
