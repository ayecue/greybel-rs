use crate::api;
use crate::protocol::{Entry, Overview};

impl Overview {
    /// Builds a table of contents from API member tags, types, and member names.
    pub(crate) fn new() -> Self {
        Self {
            tags: api::container()
                .available_tags()
                .into_iter()
                .map(str::to_owned)
                .collect(),
            contents: api::types()
                .into_iter()
                .map(|signature| {
                    let mut members: Vec<String> =
                        signature.definitions().keys().cloned().collect();

                    members.sort();

                    Entry {
                        name: signature.type_name().to_owned(),
                        members,
                    }
                })
                .collect(),
        }
    }
}
