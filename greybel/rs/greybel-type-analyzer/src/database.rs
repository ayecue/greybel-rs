use greybel_documents::SourceFile;

use crate::analysis::Analysis;
use crate::annotation::CommentTable;
use crate::program::Program;

#[salsa::db]
pub trait Db: greybel_documents::Db {}

#[salsa::db]
#[derive(Default, Clone)]
pub struct Database {
    storage: salsa::Storage<Self>,
}

impl Database {
    #[must_use]
    pub fn with_events(callback: impl Fn(salsa::Event) + Send + Sync + 'static) -> Self {
        Self {
            storage: salsa::Storage::new(Some(Box::new(callback))),
        }
    }
}

#[salsa::db]
impl salsa::Database for Database {}

#[salsa::db]
impl greybel_documents::Db for Database {}

#[salsa::db]
impl Db for Database {}

pub trait Analyze {
    fn analysis(self, db: &dyn Db) -> &Option<Analysis>;
}

pub(crate) trait Annotate {
    fn comments(self, db: &dyn Db) -> &CommentTable;
}

#[salsa::tracked]
impl Annotate for SourceFile {
    #[salsa::tracked(returns(ref))]
    fn comments(self, db: &dyn Db) -> CommentTable {
        self.parse(db)
            .ast()
            .map(|ast| CommentTable::new(ast))
            .unwrap_or_default()
    }
}

#[salsa::tracked]
impl Analyze for SourceFile {
    #[salsa::tracked(returns(ref))]
    fn analysis(self, db: &dyn Db) -> Option<Analysis> {
        Program::new(db, self).analyze(db)
    }
}
