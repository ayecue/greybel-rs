use crate::api;
use crate::protocol::{MemberPath, Search, SearchResult};

impl SearchResult {
    /// Finds case-insensitive matches in reference order and omits members with excluded tags, and types whose
    /// members all have one.
    pub(crate) fn of(search: &Search<'_>) -> Self {
        let query = search.query.to_lowercase();
        let mut result = Self {
            types: Vec::new(),
            members: Vec::new(),
        };

        for signature in api::types() {
            let owner = signature.type_name();
            let mut names: Vec<&String> = signature
                .definitions()
                .iter()
                .filter(|(_, definition)| !Self::has_excluded_tag(search, definition.tags()))
                .map(|(name, _)| name)
                .collect();

            if names.is_empty() && !signature.definitions().is_empty() {
                continue;
            }

            if owner.to_lowercase().contains(&query) {
                result.types.push(owner.to_owned());
            }

            names.sort();
            for name in names {
                if format!("{owner}.{name}").to_lowercase().contains(&query) {
                    result.members.push(MemberPath {
                        owner: owner.to_owned(),
                        name: name.clone(),
                    });
                }
            }
        }

        result
    }

    fn has_excluded_tag(search: &Search<'_>, tags: &[String]) -> bool {
        tags.iter().any(|tag| {
            search
                .excluded_tags
                .iter()
                .any(|excluded_tag| excluded_tag == tag)
        })
    }
}
