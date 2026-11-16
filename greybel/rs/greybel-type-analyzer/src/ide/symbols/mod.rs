mod document;
mod workspace;

pub use document::{DocumentSymbol, SymbolKind};
use greybel_documents::Uri;
use workspace::SymbolQuery;
pub use workspace::WorkspaceSymbol;

use crate::analyzer::Analyzer;
use crate::database::Annotate;

impl Analyzer<'_> {
    #[must_use]
    pub fn document_symbols(&self, uri: &Uri) -> Vec<DocumentSymbol> {
        let Some(file) = self.workspace().file(uri) else {
            return Vec::new();
        };

        let db = self.workspace().db();

        file.parse(db)
            .ast()
            .map(|ast| DocumentSymbol::outline_with_types(ast, file.comments(db)))
            .unwrap_or_default()
    }

    #[must_use]
    pub fn workspace_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let db = self.workspace().db();
        let query = SymbolQuery::new(query);
        let mut symbols = Vec::new();

        for file in self.workspace().files() {
            let Some(ast) = file.parse(db).ast() else {
                continue;
            };

            WorkspaceSymbol::collect(
                file.uri(db),
                DocumentSymbol::outline_with_types(ast, file.comments(db)),
                None,
                &query,
                &mut symbols,
            );
        }

        symbols.sort_by(|a, b| {
            (&a.location.uri, a.location.range.start)
                .cmp(&(&b.location.uri, b.location.range.start))
        });

        symbols
    }
}
