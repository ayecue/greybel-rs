use super::Analyzer;
use crate::ide::{DocumentSymbol, SymbolQuery, WorkspaceSymbol};

impl<'w> Analyzer<'w> {
    #[must_use]
    pub fn workspace_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        let db = self.workspace.db();
        let query = SymbolQuery::new(query);
        let mut symbols = Vec::new();

        for file in self.workspace.files() {
            let Some(ast) = file.parse(db).ast() else {
                continue;
            };

            WorkspaceSymbol::collect(
                file.uri(db),
                DocumentSymbol::outline(ast),
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
