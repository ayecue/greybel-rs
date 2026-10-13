use super::{Db, ParseResult, SourceFile};
use crate::import::{ImportContainer, ImportKind};

#[salsa::tracked]
impl SourceFile {
    #[salsa::tracked(returns(ref), no_eq)]
    pub fn parse(self, db: &dyn Db) -> ParseResult {
        let text = self.text(db).as_deref().unwrap_or_default();

        ParseResult::new(&self.uri(db).label(), text, self.project(db).options(db))
    }

    #[salsa::tracked(returns(ref))]
    pub fn imports(self, db: &dyn Db) -> ImportContainer {
        let native_imports = self.project(db).options(db).native_imports;

        self.parse(db)
            .ast()
            .map(|ast| ImportContainer::of(ast, native_imports))
            .unwrap_or_default()
    }

    #[salsa::tracked(returns(ref))]
    pub fn resolved_imports(self, db: &dyn Db) -> Vec<(ImportKind, SourceFile)> {
        let links = self.links(db);

        self.imports(db)
            .iter()
            .filter_map(|import| Some((import.kind, *links.get(&import.target)?)))
            .collect()
    }
}
