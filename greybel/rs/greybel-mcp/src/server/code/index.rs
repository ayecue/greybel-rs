use greybel_core::{Position, Range};
use greybel_documents::Uri;
use greybel_type_analyzer::{DocumentSymbol, SymbolKind};

use super::{CodeError, CodeProject};

/// A document symbol with its name path, such as `App.utils.format`: the names of the symbols containing it,
/// joined with dots.
pub struct IndexedSymbol {
    pub name_path: String,
    pub kind: SymbolKind,
    pub uri: Uri,
    pub range: Range,
    pub selection_range: Range,
}

impl IndexedSymbol {
    /// Where the symbol's own name starts: the last member of a selection like `App.utils.format`.
    pub fn name_position(&self) -> Position {
        let leaf = self.name_path.rsplit('.').next().unwrap_or(&self.name_path);

        Self::leaf_position(self.selection_range, leaf)
    }

    /// The start of `leaf` when `selection` ends with it on one line, else the start of `selection`.
    pub fn leaf_position(selection: Range, leaf: &str) -> Position {
        let length = u32::try_from(leaf.chars().count()).unwrap_or(0);
        let end = selection.end;

        if end.line == selection.start.line && end.character >= selection.start.character + length {
            Position::new(end.line, end.character - length)
        } else {
            selection.start
        }
    }
}

/// The document symbols of every project file, in file and source order.
pub struct SymbolIndex {
    symbols: Vec<IndexedSymbol>,
}

impl SymbolIndex {
    const CANDIDATE_LIMIT: usize = 10;

    pub fn new(project: &CodeProject) -> Self {
        let analyzer = project.analyzer();
        let mut symbols = Vec::new();

        for uri in project.files() {
            Self::add(uri, None, analyzer.document_symbols(uri), &mut symbols);
        }

        Self { symbols }
    }

    /// Symbols whose name contains `query`, ignoring case, or whose name path does when `query` contains a dot;
    /// within `file` when given. Each name path is listed once, at its first assignment.
    pub fn search<'a>(&'a self, query: &str, file: Option<&'a Uri>) -> Vec<&'a IndexedSymbol> {
        let query = query.to_lowercase();
        let matches_path = query.contains('.');
        let mut listed_paths = std::collections::HashSet::new();

        self.symbols
            .iter()
            .filter(|symbol| file.is_none_or(|file| symbol.uri == *file))
            .filter(|symbol| {
                let name = if matches_path {
                    symbol.name_path.as_str()
                } else {
                    symbol
                        .name_path
                        .rsplit('.')
                        .next()
                        .unwrap_or(&symbol.name_path)
                };

                name.to_lowercase().contains(&query)
            })
            .filter(|symbol| listed_paths.insert(symbol.name_path.as_str()))
            .collect()
    }

    /// The first symbol whose name path is `name`, or else ends with `.name`; within `file` when given. Name
    /// paths that match only by their end must all be the same.
    pub fn resolve(&self, name: &str, file: Option<&Uri>) -> Result<&IndexedSymbol, CodeError> {
        let in_file = |symbol: &&IndexedSymbol| file.is_none_or(|file| symbol.uri == *file);

        if let Some(symbol) = self
            .symbols
            .iter()
            .filter(in_file)
            .find(|symbol| symbol.name_path == name)
        {
            return Ok(symbol);
        }

        let suffix = format!(".{name}");
        let matches: Vec<&IndexedSymbol> = self
            .symbols
            .iter()
            .filter(in_file)
            .filter(|symbol| symbol.name_path.ends_with(&suffix))
            .collect();

        match matches.as_slice() {
            [] => Err(CodeError::UnknownSymbol(name.to_owned())),
            [first, others @ ..]
                if others
                    .iter()
                    .all(|symbol| symbol.name_path == first.name_path) =>
            {
                Ok(first)
            }
            _ => {
                let mut candidates: Vec<String> = matches
                    .iter()
                    .map(|symbol| {
                        format!(
                            "{} at {}",
                            symbol.name_path,
                            CodeProject::place(&symbol.uri, symbol.name_position())
                        )
                    })
                    .collect();

                candidates.dedup();
                candidates.truncate(Self::CANDIDATE_LIMIT);

                Err(CodeError::AmbiguousSymbol {
                    name: name.to_owned(),
                    candidates: candidates.join("\n"),
                })
            }
        }
    }

    fn add(
        uri: &Uri,
        container: Option<&str>,
        symbols: Vec<DocumentSymbol>,
        index: &mut Vec<IndexedSymbol>,
    ) {
        for symbol in symbols {
            let name_path = match container {
                Some(container) => format!("{container}.{}", symbol.name),
                None => symbol.name.clone(),
            };

            index.push(IndexedSymbol {
                name_path: name_path.clone(),
                kind: symbol.kind,
                uri: uri.clone(),
                range: symbol.range,
                selection_range: symbol.selection_range,
            });
            Self::add(uri, Some(&name_path), symbol.children, index);
        }
    }
}
