use std::collections::HashMap;

use meta_utils::Container;

use crate::protocol::Link;

/// Resolves description links to type or member anchors, page anchors, or external pages. Type anchors use uppercase
/// names (`#SHELL`); member anchors use `#SHELL_HOST_COMPUTER`.
pub(crate) struct AnchorTable {
    links: HashMap<String, Link>,
}

impl AnchorTable {
    /// Generates uppercase anchors for all types and members in `container`.
    pub(crate) fn of(container: &Container) -> Self {
        let mut links = HashMap::new();

        for signature in container.signatures() {
            let owner = signature.type_name();

            links.insert(
                owner.to_uppercase(),
                Link::Type {
                    name: owner.to_owned(),
                },
            );
            for name in signature.definitions().keys() {
                links.insert(
                    format!("{}_{}", owner.to_uppercase(), name.to_uppercase()),
                    Link::Member {
                        owner: owner.to_owned(),
                        name: name.clone(),
                    },
                );
            }
        }

        Self { links }
    }

    /// Maps a known `#` anchor in `url` to a type or member link, another `#` anchor to a page link, and HTTP(S) to
    /// a web link. Other URLs return `None`.
    pub(crate) fn resolve(&self, url: &str) -> Option<Link> {
        if let Some(anchor) = url.strip_prefix('#') {
            return Some(
                self.links
                    .get(anchor)
                    .cloned()
                    .unwrap_or_else(|| Link::Page {
                        anchor: anchor.to_owned(),
                    }),
            );
        }

        (url.starts_with("https://") || url.starts_with("http://")).then(|| Link::Web {
            url: url.to_owned(),
        })
    }
}
