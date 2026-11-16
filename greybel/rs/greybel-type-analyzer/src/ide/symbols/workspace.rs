use greybel_documents::Uri;

use super::{DocumentSymbol, SymbolKind};
use crate::ide::Location;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub container: Option<String>,
}

impl WorkspaceSymbol {
    pub(super) fn collect(
        uri: &Uri,
        symbols: Vec<DocumentSymbol>,
        container: Option<&str>,
        query: &SymbolQuery,
        results: &mut Vec<Self>,
    ) {
        for symbol in symbols {
            if query.matches(&symbol.name) {
                results.push(Self {
                    name: symbol.name.clone(),
                    kind: symbol.kind,
                    location: Location {
                        uri: uri.clone(),
                        range: symbol.selection_range,
                    },
                    container: container.map(str::to_owned),
                });
            }

            Self::collect(uri, symbol.children, Some(&symbol.name), query, results);
        }
    }
}

pub(super) struct SymbolQuery {
    characters: Vec<char>,
}

impl SymbolQuery {
    pub(super) fn new(query: &str) -> Self {
        Self {
            characters: query.chars().flat_map(char::to_lowercase).collect(),
        }
    }

    fn matches(&self, name: &str) -> bool {
        let mut query_chars = self.characters.iter().peekable();

        for c in name.chars().flat_map(char::to_lowercase) {
            if query_chars.peek() == Some(&&c) {
                query_chars.next();
            }
        }

        query_chars.peek().is_none()
    }
}
