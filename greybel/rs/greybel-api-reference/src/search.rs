use crate::api;
use crate::protocol::{MemberPath, Search, SearchResult};

impl SearchResult {
    /// Finds case-insensitive matches in reference order and omits members with excluded tags.
    pub(crate) fn of(search: &Search<'_>) -> Self {
        let query = search.query.to_lowercase();
        let mut result = Self {
            types: Vec::new(),
            members: Vec::new(),
        };

        for signature in api::types() {
            let owner = signature.type_name();

            if owner.to_lowercase().contains(&query) {
                result.types.push(owner.to_owned());
            }

            let mut names: Vec<&String> = signature.definitions().keys().collect();

            names.sort();
            for name in names {
                let is_excluded = signature.definitions()[name].tags().iter().any(|tag| {
                    search
                        .excluded_tags
                        .iter()
                        .any(|excluded_tag| excluded_tag == tag)
                });

                if !is_excluded && format!("{owner}.{name}").to_lowercase().contains(&query) {
                    result.members.push(MemberPath {
                        owner: owner.to_owned(),
                        name: name.clone(),
                    });
                }
            }
        }

        result
    }
}
